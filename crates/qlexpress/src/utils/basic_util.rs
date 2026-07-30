//! Basic helpers, mirroring Java `BasicUtil`.
//!
//! Java's `Class<?>`-keyed maps are replaced by small Rust enums since Rust
//! has no runtime class objects (SPEC §4).

use crate::runtime::class_ref::ClassRef;
use crate::runtime::value::DataValue;

/// Well-known member name: Java `BasicUtil.LENGTH`.
pub const LENGTH: &str = "length";

/// Well-known member name: Java `BasicUtil.CLASS`.
pub const CLASS: &str = "class";

pub use super::num_kind::NumKind;
pub use super::primitive_type::PrimitiveType;

impl NumKind {
    /// 返回数值类型参与 Java 运算时的提升等级。
    /// 无显式参数；返回：`u8`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/BasicUtil.java`，方法 `promoteLevel`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `BasicUtil.numberPromoteLevel`:
    /// byte=0, short=1, int=2, long=3, BigInteger=4, float=5, double=6,
    /// BigDecimal=7.
    /// 对应 Java: com.alibaba.qlexpress4.utils.BasicUtil#promoteLevel。
    pub fn promote_level(self) -> u8 {
        match self {
            NumKind::Byte => 0,
            NumKind::Short => 1,
            NumKind::Int => 2,
            NumKind::Long => 3,
            NumKind::BigInteger => 4,
            NumKind::Float => 5,
            NumKind::Double => 6,
            NumKind::BigDecimal => 7,
        }
    }
}

/// 把包装类型名转换为对应 Java 原语类型名。
/// 参数：`primitive`；返回：`PrimitiveType`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/BasicUtil.java`，方法 `transToPrimitive`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `BasicUtil.transToPrimitive`: in Rust boxed and primitive forms are
/// the same type, so this is the identity mapping kept for API parity.
/// 对应 Java: com.alibaba.qlexpress4.utils.BasicUtil#transToPrimitive。
pub fn trans_to_primitive(primitive: PrimitiveType) -> PrimitiveType {
    primitive
}

/// 生成 Java Bean getter/setter 方法名。
/// 对应 Java: `com.alibaba.qlexpress4.utils.BasicUtil`。
pub struct BasicUtil;

impl BasicUtil {
    /// 返回一组运行时对象的 Java 类型。
    ///
    /// 对应 Java：`BasicUtil#getTypeOfObject(Object[])`。Java `null` 映射为
    /// `Nothing.class`；宿主对象读取显式注册的规范类型名；数组在元素类型
    /// 一致时保留组件类型，否则退化为 `Object[]`。
    ///
    /// # 参数
    ///
    /// - `objects`：运行时脚本值。
    ///
    /// # 返回值
    ///
    /// 返回与输入等长的 [`ClassRef`] 列表。
    pub fn get_type_of_object(objects: &[DataValue]) -> Vec<ClassRef> {
        objects.iter().map(Self::type_of_value).collect()
    }

    /// 返回一个运行时脚本值的 Java 类型。
    ///
    /// 这是 `getTypeOfObject` 单元素分派的 Rust 形态，并由成员/构造器
    /// 重载解析共用。
    pub fn type_of_value(value: &DataValue) -> ClassRef {
        match value {
            DataValue::Null => {
                ClassRef::Named("com.alibaba.qlexpress4.runtime.Nothing".to_string())
            }
            DataValue::Array(values) => {
                let values = values.borrow();
                ClassRef::array_of(values.component_type().clone())
            }
            DataValue::Object(object) => {
                ClassRef::Named(object.borrow().native_type_name().to_string())
            }
            DataValue::Lambda(_) => {
                ClassRef::Named("com.alibaba.qlexpress4.runtime.QLambda".to_string())
            }
            _ => ClassRef::from_name(value.data_type_name()),
        }
    }

    /// 查询 getter。
    /// 参数：`s`；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/BasicUtil.java`，方法 `getGetter`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `BasicUtil.getGetter`: `"get" + Capitalized(s)`.
    pub fn get_getter(s: &str) -> String {
        capitalize_prefixed("get", s)
    }

    /// 查询 setter。
    /// 参数：`s`；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/BasicUtil.java`，方法 `getSetter`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `BasicUtil.getSetter`: `"set" + Capitalized(s)`.
    /// 对应 Java: com.alibaba.qlexpress4.utils.BasicUtil#getSetter。
    pub fn get_setter(s: &str) -> String {
        capitalize_prefixed("set", s)
    }

    /// 查询 is getter。
    /// 参数：`s`；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/BasicUtil.java`，方法 `getIsGetter`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `BasicUtil.getIsGetter`: `"is" + Capitalized(s)`.
    /// 对应 Java: com.alibaba.qlexpress4.utils.BasicUtil#getIsGetter。
    pub fn get_is_getter(s: &str) -> String {
        capitalize_prefixed("is", s)
    }
}

/// Java `Character.toUpperCase(s.charAt(0)) + s.substring(1)` behind a
/// prefix. ASCII/Unicode-aware uppercasing of the first char.
fn capitalize_prefixed(prefix: &str, s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => {
            let mut result = String::with_capacity(prefix.len() + s.len());
            result.push_str(prefix);
            result.extend(first.to_uppercase());
            result.extend(chars);
            result
        }
        // Java would throw StringIndexOutOfBoundsException on empty input;
        // returning the prefix keeps this total and panic-free.
        None => prefix.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promote_levels_match_java() {
        assert_eq!(NumKind::Byte.promote_level(), 0);
        assert_eq!(NumKind::Short.promote_level(), 1);
        assert_eq!(NumKind::Int.promote_level(), 2);
        assert_eq!(NumKind::Long.promote_level(), 3);
        assert_eq!(NumKind::BigInteger.promote_level(), 4);
        assert_eq!(NumKind::Float.promote_level(), 5);
        assert_eq!(NumKind::Double.promote_level(), 6);
        assert_eq!(NumKind::BigDecimal.promote_level(), 7);
    }

    #[test]
    fn getter_setter_names() {
        assert_eq!(BasicUtil::get_getter("name"), "getName");
        assert_eq!(BasicUtil::get_setter("name"), "setName");
        assert_eq!(BasicUtil::get_is_getter("empty"), "isEmpty");
    }

    /// SOURCE_PARITY: BasicUtil#getTypeOfObject(Object[])。
    #[test]
    fn type_of_object_preserves_null_scalar_and_array_types() {
        let types = BasicUtil::get_type_of_object(&[
            DataValue::Null,
            DataValue::Int(1),
            DataValue::array(vec![DataValue::Str("a".into()), DataValue::Str("b".into())]),
            DataValue::array(vec![DataValue::Int(1), DataValue::Str("b".into())]),
        ]);
        assert_eq!(
            types,
            vec![
                ClassRef::Named("com.alibaba.qlexpress4.runtime.Nothing".into()),
                ClassRef::Boxed(
                    crate::runtime::data::convert::obj_type_convertor::TargetType::Int,
                ),
                ClassRef::from_name("java.lang.String[]"),
                ClassRef::from_name("java.lang.Object[]"),
            ]
        );
    }
}
