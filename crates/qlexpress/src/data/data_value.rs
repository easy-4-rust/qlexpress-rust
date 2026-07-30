//! 具体数据值。对应 Java
//! `com.alibaba.qlexpress4.runtime.data.DataValue`。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use num_bigint::BigInt;

use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::java_array::JavaArray;
use crate::runtime::data::java_array_list::JavaArrayList;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::native_object::NativeObject;
use crate::runtime::qlambda::QLambda;
use crate::runtime::value::Value;

/// 保存具体数据的脚本值。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.data.DataValue`。`BigInt`
/// 使用任意精度整数，对齐 Java `BigInteger`；共享集合通过 `Rc<RefCell>`
/// 保留 Java 引用语义。
#[derive(Clone)]
pub enum DataValue {
    /// Java `Value.NULL_VALUE` 的内部值。
    Null,
    /// Java `Boolean`。
    Bool(bool),
    /// Java `Byte`。
    Byte(i8),
    /// Java `Short`。
    Short(i16),
    /// Java `Integer`。
    Int(i32),
    /// Java `Long`。
    Long(i64),
    /// Java `Float`。
    Float(f32),
    /// Java `Double`。
    Double(f64),
    /// Java `BigInteger` 的任意精度表示。
    BigInt(BigInt),
    /// Java `BigDecimal` 的十进制文本表示。
    BigDec(String),
    /// Java `Character`（单个 UTF-16 code unit，可表示 surrogate）。
    Char(u16),
    /// Java `String`。
    Str(String),
    /// Java `ArrayList` 引用。
    List(Rc<RefCell<JavaArrayList>>),
    /// Java `LinkedHashMap` 引用。
    Map(Rc<RefCell<IndexMap>>),
    /// Java 数组引用。
    Array(Rc<RefCell<JavaArray>>),
    /// Java `QLambda`。
    Lambda(Rc<QLambda>),
    /// 显式注册的宿主对象。
    Object(Rc<RefCell<dyn NativeObject>>),
}

impl DataValue {
    /// Java 常量 `Value.NULL_VALUE`。
    pub const NULL_VALUE: DataValue = DataValue::Null;

    /// 判断是否为 Java `null`。对应 `DataValue#get() == null`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#isNull。
    pub fn is_null(&self) -> bool {
        matches!(self, DataValue::Null)
    }

    /// 判断是否为 Java `Number`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#isNumber。
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            DataValue::Byte(_)
                | DataValue::Short(_)
                | DataValue::Int(_)
                | DataValue::Long(_)
                | DataValue::Float(_)
                | DataValue::Double(_)
                | DataValue::BigInt(_)
                | DataValue::BigDec(_)
        )
    }

    /// 返回 Java 类风格类型名。对应 Java 方法 `Value#getTypeName()`。
    pub fn data_type_name(&self) -> &'static str {
        match self {
            DataValue::Null => "com.alibaba.qlexpress4.runtime.Nothing",
            DataValue::Bool(_) => "java.lang.Boolean",
            DataValue::Byte(_) => "java.lang.Byte",
            DataValue::Short(_) => "java.lang.Short",
            DataValue::Int(_) => "java.lang.Integer",
            DataValue::Long(_) => "java.lang.Long",
            DataValue::Float(_) => "java.lang.Float",
            DataValue::Double(_) => "java.lang.Double",
            DataValue::BigInt(_) => "java.math.BigInteger",
            DataValue::BigDec(_) => "java.math.BigDecimal",
            DataValue::Char(_) => "java.lang.Character",
            DataValue::Str(_) => "java.lang.String",
            DataValue::List(_) => "java.util.ArrayList",
            DataValue::Map(_) => "java.util.LinkedHashMap",
            DataValue::Array(_) => "java.lang.Object[]",
            DataValue::Lambda(_) => "com.alibaba.qlexpress4.runtime.QLambda",
            DataValue::Object(_) => "com.alibaba.qlexpress4.NativeObject",
        }
    }

    /// 返回实际 Java 运行时类型名。
    ///
    /// 与 [`Self::data_type_name`] 的静态枚举标签不同，宿主对象返回
    /// [`NativeObject::native_type_name`](crate::runtime::native_object::NativeObject::native_type_name)；
    /// 对应 Java `value.getClass().getName()`。
    pub fn runtime_type_name(&self) -> String {
        match self {
            DataValue::Object(object) => object.borrow().native_type_name().to_string(),
            DataValue::Array(array) => {
                format!("{}[]", array.borrow().component_type().java_name())
            }
            _ => self.data_type_name().to_string(),
        }
    }

    /// 读取布尔值；非布尔返回 `None`。Rust 便捷方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#asBool。
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DataValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// 读取字符串切片；非字符串返回 `None`。Rust 便捷方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#asStr。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DataValue::Str(value) => Some(value),
            _ => None,
        }
    }

    /// 创建 Java `ArrayList` 值。Rust 便捷构造方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#list。
    pub fn list(items: Vec<DataValue>) -> DataValue {
        DataValue::List(Rc::new(RefCell::new(JavaArrayList::new(items))))
    }

    /// 创建 Java 数组值。Rust 便捷构造方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#array。
    pub fn array(items: Vec<DataValue>) -> DataValue {
        let component_type = items
            .first()
            .filter(|first| !first.is_null())
            .map(|first| ClassRef::from_name(&first.runtime_type_name()))
            .filter(|first_type| {
                items.iter().skip(1).all(|item| {
                    ClassRef::from_name(&item.runtime_type_name()) == *first_type
                })
            })
            .unwrap_or_else(|| ClassRef::Named("java.lang.Object".to_string()));
        DataValue::array_with_component(items, component_type)
    }

    /// 创建无需宿主继承查询的声明组件类型数组。
    pub fn array_with_component(items: Vec<DataValue>, component_type: ClassRef) -> DataValue {
        DataValue::Array(Rc::new(RefCell::new(JavaArray::typed_without_registry(
            items,
            component_type,
        ))))
    }

    /// 创建携带完整 Java 声明组件类型的数组值。
    pub fn array_with_type(
        items: Vec<DataValue>,
        component_type: ClassRef,
        type_registry: Rc<NativeRegistry>,
    ) -> DataValue {
        DataValue::Array(Rc::new(RefCell::new(JavaArray::typed(
            items,
            component_type,
            type_registry,
        ))))
    }

    /// 创建 Java `LinkedHashMap` 值。Rust 便捷构造方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#map。
    pub fn map(map: IndexMap) -> DataValue {
        DataValue::Map(Rc::new(RefCell::new(map)))
    }

    /// 从整数创建 Java `BigInteger` 值。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#bigInt。
    pub fn big_int(value: impl Into<BigInt>) -> DataValue {
        DataValue::BigInt(value.into())
    }

    /// 判断是否为宿主对象。Rust 便捷方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#isObject。
    pub fn is_object(&self) -> bool {
        matches!(self, DataValue::Object(_))
    }

    /// 借用宿主对象引用。Rust 宿主集成便捷方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#asObjectRef。
    pub fn as_object_ref(&self) -> Option<&Rc<RefCell<dyn NativeObject>>> {
        match self {
            DataValue::Object(reference) => Some(reference),
            _ => None,
        }
    }

    /// 将宿主对象向下转换为具体 Rust 类型。Rust 宿主集成便捷方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#downcastObject。
    pub fn downcast_object<T: 'static>(&self) -> Option<Rc<RefCell<T>>> {
        use std::any::Any;
        let reference = self.as_object_ref()?;
        (reference as &dyn Any)
            .downcast_ref::<Rc<RefCell<T>>>()
            .cloned()
    }

    /// 按 Java `String.valueOf` 规则渲染值。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.DataValue#stringValueOf。
    pub fn string_value_of(&self) -> String {
        match self {
            DataValue::Null => "null".to_string(),
            DataValue::Bool(value) => value.to_string(),
            DataValue::Byte(value) => value.to_string(),
            DataValue::Short(value) => value.to_string(),
            DataValue::Int(value) => value.to_string(),
            DataValue::Long(value) => value.to_string(),
            DataValue::Float(value) => {
                if value.fract() == 0.0 && value.is_finite() {
                    format!("{value:.1}")
                } else {
                    value.to_string()
                }
            }
            DataValue::Double(value) => {
                if value.fract() == 0.0 && value.is_finite() {
                    format!("{value:.1}")
                } else {
                    value.to_string()
                }
            }
            DataValue::BigInt(value) => value.to_string(),
            DataValue::BigDec(value) => value.clone(),
            DataValue::Char(value) => String::from_utf16_lossy(&[*value]),
            DataValue::Str(value) => value.clone(),
            DataValue::List(value) => format!(
                "[{}]",
                value
                    .borrow()
                    .iter()
                    .map(DataValue::string_value_of)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            DataValue::Map(value) => format!(
                "{{{}}}",
                value
                    .borrow()
                    .entries()
                    .iter()
                    .map(|(key, value)| format!(
                        "{}={}",
                        key.string_value_of(),
                        value.string_value_of()
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            other => format!("{other:?}"),
        }
    }
}

impl Value for DataValue {
    fn get(&self) -> DataValue {
        self.clone()
    }

    fn type_name(&self) -> &'static str {
        self.data_type_name()
    }
}

/// QLExpress 相等语义：数值跨类型提升后比较，集合结构比较，
/// Lambda/宿主对象按引用身份比较。
impl PartialEq for DataValue {
    fn eq(&self, other: &Self) -> bool {
        use DataValue::{Array, Bool, Char, Lambda, List, Map, Null, Object, Str};
        match (self, other) {
            (Null, Null) => true,
            (Bool(left), Bool(right)) => left == right,
            (Char(left), Char(right)) => left == right,
            (Str(left), Str(right)) => left == right,
            (List(left), List(right)) => {
                Rc::ptr_eq(left, right) || *left.borrow() == *right.borrow()
            }
            // Java 数组沿用 Object.equals，按引用身份比较。
            (Array(left), Array(right)) => Rc::ptr_eq(left, right),
            (Map(left), Map(right)) => Rc::ptr_eq(left, right) || *left.borrow() == *right.borrow(),
            (Lambda(left), Lambda(right)) => Rc::ptr_eq(left, right),
            (Object(left), Object(right)) => {
                if Rc::ptr_eq(left, right) {
                    true
                } else {
                    match (
                        crate::runtime::meta_class::as_meta_class(self),
                        crate::runtime::meta_class::as_meta_class(other),
                    ) {
                        (Some(left_class), Some(right_class)) => left_class == right_class,
                        _ => false,
                    }
                }
            }
            _ if self.is_number() && other.is_number() => {
                crate::runtime::data::convert::number_compare(self, other)
                    == Some(std::cmp::Ordering::Equal)
            }
            _ => false,
        }
    }
}

impl fmt::Debug for DataValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataValue::Null => write!(formatter, "Null"),
            DataValue::Bool(value) => write!(formatter, "Bool({value})"),
            DataValue::Byte(value) => write!(formatter, "Byte({value})"),
            DataValue::Short(value) => write!(formatter, "Short({value})"),
            DataValue::Int(value) => write!(formatter, "Int({value})"),
            DataValue::Long(value) => write!(formatter, "Long({value})"),
            DataValue::Float(value) => write!(formatter, "Float({value})"),
            DataValue::Double(value) => write!(formatter, "Double({value})"),
            DataValue::BigInt(value) => write!(formatter, "BigInt({value})"),
            DataValue::BigDec(value) => write!(formatter, "BigDec({value})"),
            DataValue::Char(value) => write!(formatter, "Char({value:?})"),
            DataValue::Str(value) => write!(formatter, "Str({value:?})"),
            DataValue::List(value) => formatter
                .debug_tuple("List")
                .field(&value.borrow())
                .finish(),
            DataValue::Map(value) => formatter.debug_tuple("Map").field(&value.borrow()).finish(),
            DataValue::Array(value) => formatter
                .debug_tuple("Array")
                .field(&value.borrow())
                .finish(),
            DataValue::Lambda(value) => formatter.debug_tuple("Lambda").field(value).finish(),
            DataValue::Object(value) => {
                write!(formatter, "Object({})", value.borrow().native_type_name())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_equality_promotes_across_types() {
        assert_eq!(DataValue::Int(1), DataValue::Long(1));
        assert_eq!(DataValue::Int(1), DataValue::Double(1.0));
        assert_eq!(DataValue::Byte(1), DataValue::Short(1));
        assert_eq!(DataValue::BigDec("1.00".to_string()), DataValue::Int(1));
        assert_ne!(DataValue::Int(1), DataValue::Int(2));
        assert_ne!(DataValue::Int(1), DataValue::Bool(true));
    }

    #[test]
    fn structural_and_identity_equality() {
        assert_eq!(
            DataValue::list(vec![DataValue::Int(1), DataValue::Str("a".into())]),
            DataValue::list(vec![DataValue::Long(1), DataValue::Str("a".into())])
        );
        assert_eq!(DataValue::Null, DataValue::Null);
        assert_ne!(DataValue::Null, DataValue::Int(0));
        assert_eq!(
            DataValue::Char('a' as u16),
            DataValue::Char('a' as u16)
        );
        assert_ne!(DataValue::Char('a' as u16), DataValue::Str("a".into()));
    }

    #[test]
    fn string_value_of_collections_matches_java_to_string_shape() {
        let list = DataValue::list(vec![
            DataValue::Int(1),
            DataValue::Str("two".to_string()),
            DataValue::Null,
        ]);
        assert_eq!(list.string_value_of(), "[1, two, null]");

        let map = DataValue::map(IndexMap::from_entries(vec![
            (
                DataValue::Str("name".to_string()),
                DataValue::Str("QlExpress Rust".to_string()),
            ),
            (DataValue::Str("items".to_string()), list),
        ]));
        assert_eq!(
            map.string_value_of(),
            "{name=QlExpress Rust, items=[1, two, null]}"
        );
    }

    #[test]
    fn type_names_match_java_class_names() {
        assert_eq!(DataValue::Int(0).type_name(), "java.lang.Integer");
        assert_eq!(
            DataValue::Null.type_name(),
            "com.alibaba.qlexpress4.runtime.Nothing"
        );
        assert_eq!(DataValue::big_int(0).type_name(), "java.math.BigInteger");
    }
}
