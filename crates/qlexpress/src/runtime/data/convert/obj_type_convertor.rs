//! Object→type conversion, mirroring Java `ObjTypeConvertor`.
//!
//! Java keys conversions off `Class<?>` objects; Rust uses [`TargetType`].
//! The Java function-interface proxy branch is handled by the native
//! registry (SPEC §4/§6), not here.

use crate::runtime::value::DataValue;

use super::{to_big_dec_string, to_big_int, to_f64, to_i64};

/// `TargetType` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/DefaultClassSupplier.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Conversion target, standing in for the Java `Class<?>` parameter of
/// `ObjTypeConvertor.cast`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.runtime.data.convert.ObjTypeConvertor。
pub enum TargetType {
    /// Java `Boolean` 类型或语义类别。
    Boolean,
    /// Java `Byte` 类型或语义类别。
    Byte,
    /// Java `Short` 类型或语义类别。
    Short,
    /// Java `Int` 类型或语义类别。
    Int,
    /// Java `Long` 类型或语义类别。
    Long,
    /// Java `Float` 类型或语义类别。
    Float,
    /// Java `Double` 类型或语义类别。
    Double,
    /// Java `BigInteger` 类型或语义类别。
    BigInteger,
    /// Java `BigDecimal` 类型或语义类别。
    BigDecimal,
    /// Java `Character` 类型或语义类别。
    Character,
    /// Java `Object.class` — accepts any value unchanged.
    Any,
}

impl TargetType {
    /// 处理 java name 对应的领域职责。
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

/// `QConverted` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/convert/ObjTypeConvertor.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Result of a conversion attempt, mirroring Java
/// `ObjTypeConvertor.QConverted`.
#[derive(Clone, Debug, PartialEq)]
/// 对应 Java: com.alibaba.qlexpress4.runtime.data.convert.ObjTypeConvertor。
pub struct QConverted {
    convertible: bool,
    /// The converted value; `DataValue::Null` when not convertible
    /// (Java stores `null`).
    converted: DataValue,
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
    pub fn into_converted(self) -> DataValue {
        self.converted
    }
}

/// 执行单个脚本值到宿主目标类型的转换，并显式记录不可转换状态。
/// 对应 Java: `com.alibaba.qlexpress4.runtime.data.convert.ObjTypeConvertor`。
pub struct ObjTypeConvertor;

impl ObjTypeConvertor {
    /// 处理 cast 对应的领域职责。
    /// 参数：`value`、`target`；返回：`QConverted`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/convert/ObjTypeConvertor.java`，方法 `cast`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `ObjTypeConvertor.cast(Object, Class<?>)`.
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
                v if v.is_number() => QConverted::converted(DataValue::BigInt(to_big_int(v))),
                _ => QConverted::un_convertible(),
            },
            TargetType::BigDecimal => match value {
                v if v.is_number() => {
                    QConverted::converted(DataValue::BigDec(to_big_dec_string(v)))
                }
                _ => QConverted::un_convertible(),
            },
            // `Any` is handled by `no_need_convert`; unreachable here.
            TargetType::Any => QConverted::un_convertible(),
        }
    }

    /// 处理 cast opt 对应的领域职责。
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

    /// Java `castChar`.
    fn cast_char(value: &DataValue) -> QConverted {
        match value {
            DataValue::Char(_) => QConverted::converted(value.clone()),
            v if v.is_number() => {
                // Java `(char) number.intValue()`: low 16 bits as UTF-16 unit.
                let code = to_i64(v) as i32 as u32 & 0xFFFF;
                match char::from_u32(code) {
                    Some(c) => QConverted::converted(DataValue::Char(c)),
                    None => QConverted::un_convertible(),
                }
            }
            DataValue::Str(s) => {
                let mut chars = s.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => QConverted::converted(DataValue::Char(c)),
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
            &DataValue::Char('a')
        );
        assert!(
            !ObjTypeConvertor::cast(&DataValue::Str("ab".into()), TargetType::Character)
                .is_convertible()
        );
        assert_eq!(
            ObjTypeConvertor::cast(&DataValue::Int(97), TargetType::Character).get_converted(),
            &DataValue::Char('a')
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
