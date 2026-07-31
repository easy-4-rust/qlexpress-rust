//! Object→type conversion, mirroring Java `ObjTypeConvertor`.
//!
//! Java keys conversions off `Class<?>` objects; Rust uses [`TargetType`].
//! The Java function-interface proxy branch is handled by the native
//! registry (SPEC §4/§6), not here.

use crate::runtime::class_ref::ClassRef;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::value::DataValue;

use super::{to_big_dec_string, to_f64, to_i64, try_to_big_int};

pub use super::q_converted::QConverted;
pub use super::target_type::TargetType;

impl TargetType {
    /// 返回与 Java 语义一致的规范名称。
    /// 无显式参数；返回：`&'static str`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/convert/ObjTypeConvertor.java`，方法 `javaName`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `Class.getName()`-style display, used in error messages.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.convert.ObjTypeConvertor#javaName。
    pub fn java_name(self) -> &'static str {
        match self {
            TargetType::Boolean => "java.lang.Boolean",
            TargetType::Byte => "java.lang.Byte",
            TargetType::Short => "java.lang.Short",
            TargetType::Int => "java.lang.Integer",
            TargetType::Long => "java.lang.Long",
            TargetType::Float => "java.lang.Float",
            TargetType::Double => "java.lang.Double",
            TargetType::BigInteger => "java.math.BigInteger",
            TargetType::BigDecimal => "java.math.BigDecimal",
            TargetType::Character => "java.lang.Character",
            TargetType::Any => "java.lang.Object",
        }
    }
}

impl QConverted {
    /// 创建转换成功结果并保存目标值。
    /// 对应 Java: `new QConverted(true, converted)`。
    pub fn converted(converted: DataValue) -> Self {
        QConverted {
            convertible: true,
            converted,
        }
    }

    /// 创建不可转换结果，转换值按 Java `null` 语义置为 `DataValue::Null`。
    /// 对应 Java：`QConverted#unConvertable()`。
    pub fn un_convertible() -> Self {
        QConverted {
            convertible: false,
            converted: DataValue::Null,
        }
    }

    /// 返回本次类型转换是否成功。对应 Java: `QConverted#isConvertible`。
    pub fn is_convertible(&self) -> bool {
        self.convertible
    }

    /// 借用转换后的值。对应 Java: `QConverted#getConverted`。
    pub fn get_converted(&self) -> &DataValue {
        &self.converted
    }

    /// 消费结果并取得转换后的值；不可转换时得到 `DataValue::Null`。
    /// 对应 Java：`QConverted#getConverted()`（Rust 所有权便捷接口）。
    pub fn into_converted(self) -> DataValue {
        self.converted
    }
}

/// 执行单个脚本值到宿主目标类型的转换，并显式记录不可转换状态。
/// 对应 Java: `com.alibaba.qlexpress4.runtime.data.convert.ObjTypeConvertor`。
pub struct ObjTypeConvertor;

impl ObjTypeConvertor {
    /// 按 Java 类型转换规则转换输入值。
    /// 参数：`value`、`target`；返回：`QConverted`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/convert/ObjTypeConvertor.java`，方法 `cast`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `ObjTypeConvertor.cast(Object, Class<?>)`.
    /// 对应 Java：`ObjTypeConvertor#cast(Object,Class<?>)`。
    pub fn cast(value: &DataValue, target: TargetType) -> QConverted {
        if no_need_convert(value, target) {
            return QConverted::converted(value.clone());
        }
        match target {
            TargetType::Character => Self::cast_char(value),
            TargetType::Boolean => QConverted::un_convertible(),
            TargetType::Byte => match value {
                DataValue::Byte(_) => QConverted::converted(value.clone()),
                v if v.is_number() => QConverted::converted(DataValue::Byte(to_i64(v) as i8)),
                _ => QConverted::un_convertible(),
            },
            TargetType::Short => match value {
                DataValue::Short(_) => QConverted::converted(value.clone()),
                v if v.is_number() => QConverted::converted(DataValue::Short(to_i64(v) as i16)),
                _ => QConverted::un_convertible(),
            },
            TargetType::Int => match value {
                DataValue::Int(_) => QConverted::converted(value.clone()),
                v if v.is_number() => QConverted::converted(DataValue::Int(to_i64(v) as i32)),
                _ => QConverted::un_convertible(),
            },
            TargetType::Long => match value {
                DataValue::Long(_) => QConverted::converted(value.clone()),
                v if v.is_number() => QConverted::converted(DataValue::Long(to_i64(v))),
                _ => QConverted::un_convertible(),
            },
            TargetType::Float => match value {
                DataValue::Float(_) => QConverted::converted(value.clone()),
                v if v.is_number() => QConverted::converted(DataValue::Float(to_f64(v) as f32)),
                _ => QConverted::un_convertible(),
            },
            TargetType::Double => match value {
                DataValue::Double(_) => QConverted::converted(value.clone()),
                v if v.is_number() => QConverted::converted(DataValue::Double(to_f64(v))),
                _ => QConverted::un_convertible(),
            },
            TargetType::BigInteger => match value {
                v if v.is_number() => match try_to_big_int(v) {
                    Some(converted) => QConverted::converted(DataValue::BigInt(converted)),
                    None => QConverted::un_convertible(),
                },
                _ => QConverted::un_convertible(),
            },
            TargetType::BigDecimal => match value {
                DataValue::Float(v) if !v.is_finite() => QConverted::un_convertible(),
                DataValue::Double(v) if !v.is_finite() => QConverted::un_convertible(),
                v if v.is_number() => {
                    QConverted::converted(DataValue::BigDec(to_big_dec_string(v)))
                }
                _ => QConverted::un_convertible(),
            },
            // `Any` is handled by `no_need_convert`; unreachable here.
            TargetType::Any => QConverted::un_convertible(),
        }
    }

    /// 向可选目标类型执行 Java 兼容转换。
    /// 参数：`value`、`target`；返回：`QConverted`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/convert/ObjTypeConvertor.java`，方法 `castOpt`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `ObjTypeConvertor.cast` overload taking a nullable `Class<?>`;
    /// `None` mirrors `type == null` (no conversion needed).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.convert.ObjTypeConvertor#castOpt。
    pub fn cast_opt(value: &DataValue, target: Option<TargetType>) -> QConverted {
        match target {
            None => QConverted::converted(value.clone()),
            Some(t) => Self::cast(value, t),
        }
    }

    /// 按完整 Java `Class<?>` 引用执行转换。
    ///
    /// 原语/包装类型沿用数值与字符转换；具名引用类型按注册表中的继承关系
    /// 校验并保持原值。对应 Java `type.isInstance(value)` 分支。
    pub fn cast_class(
        value: &DataValue,
        target: Option<&ClassRef>,
        registry: Option<&NativeRegistry>,
    ) -> QConverted {
        let Some(target) = target else {
            return QConverted::converted(value.clone());
        };
        match target {
            ClassRef::Primitive(target) | ClassRef::Boxed(target) => Self::cast(value, *target),
            ClassRef::Named(_) if value.is_null() => QConverted::converted(value.clone()),
            ClassRef::Named(name) if name == "java.lang.Object" => {
                QConverted::converted(value.clone())
            }
            ClassRef::Named(_) => {
                let assignable = match registry {
                    Some(registry) => registry.is_value_assignable(target, value),
                    None => {
                        let value_type_name = value.runtime_type_name();
                        target.java_name() == value_type_name.as_str()
                    }
                };
                if assignable {
                    QConverted::converted(value.clone())
                } else {
                    QConverted::un_convertible()
                }
            }
        }
    }

    /// Java `castChar`.
    fn cast_char(value: &DataValue) -> QConverted {
        match value {
            DataValue::Char(_) => QConverted::converted(value.clone()),
            v if v.is_number() => {
                // Java `(char) number.intValue()`: low 16 bits as UTF-16 unit.
                QConverted::converted(DataValue::Char(to_i64(v) as i32 as u16))
            }
            DataValue::Str(s) => {
                let mut units = s.encode_utf16();
                match (units.next(), units.next()) {
                    (Some(unit), None) => QConverted::converted(DataValue::Char(unit)),
                    _ => QConverted::un_convertible(),
                }
            }
            _ => QConverted::un_convertible(),
        }
    }
}

/// Java `noNeedConvert`: null target, null value, or value already an
/// instance of the target type.
fn no_need_convert(value: &DataValue, target: TargetType) -> bool {
    if target == TargetType::Any || value.is_null() {
        return true;
    }
    matches!(
        (value, target),
        (DataValue::Bool(_), TargetType::Boolean)
            | (DataValue::Byte(_), TargetType::Byte)
            | (DataValue::Short(_), TargetType::Short)
            | (DataValue::Int(_), TargetType::Int)
            | (DataValue::Long(_), TargetType::Long)
            | (DataValue::Float(_), TargetType::Float)
            | (DataValue::Double(_), TargetType::Double)
            | (DataValue::BigInt(_), TargetType::BigInteger)
            | (DataValue::BigDec(_), TargetType::BigDecimal)
            | (DataValue::Char(_), TargetType::Character)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_need_convert_cases() {
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Int(3), TargetType::Int).get_converted(),
            &DataValue::Int(3)
        );
        assert!(ObjTypeConvertor::cast(&DataValue::Null, TargetType::Int).is_convertible());
        assert!(
            ObjTypeConvertor::cast(&DataValue::Str("s".into()), TargetType::Any).is_convertible()
        );
        assert!(ObjTypeConvertor::cast_opt(&DataValue::Int(1), None).is_convertible());
    }

    #[test]
    fn number_widening_and_narrowing() {
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Int(300), TargetType::Byte).get_converted(),
            &DataValue::Byte(300i64 as i8) // Java byteValue() truncation
        );
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Double(2.7), TargetType::Long).get_converted(),
            &DataValue::Long(2)
        );
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Int(5), TargetType::BigDecimal).get_converted(),
            &DataValue::BigDec("5".into())
        );
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Double(3.9), TargetType::BigInteger).get_converted(),
            &DataValue::big_int(3)
        );
    }

    #[test]
    fn unconvertible_cases() {
        assert!(
            !ObjTypeConvertor::cast(&DataValue::Str("x".into()), TargetType::Int).is_convertible()
        );
        assert!(!ObjTypeConvertor::cast(&DataValue::Int(1), TargetType::Boolean).is_convertible());
        assert!(
            !ObjTypeConvertor::cast(&DataValue::Bool(true), TargetType::Double).is_convertible()
        );
    }

    #[test]
    fn cast_char_semantics() {
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Str("a".into()), TargetType::Character)
                .get_converted(),
            &DataValue::Char('a' as u16)
        );
        assert!(
            !ObjTypeConvertor::cast(&DataValue::Str("ab".into()), TargetType::Character)
                .is_convertible()
        );
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Int(97), TargetType::Character).get_converted(),
            &DataValue::Char('a' as u16)
        );
    }

    #[test]
    fn bool_stays_bool() {
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Bool(true), TargetType::Boolean).get_converted(),
            &DataValue::Bool(true)
        );
    }
}
