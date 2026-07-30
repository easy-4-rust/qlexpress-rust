//! Basic helpers, mirroring Java `BasicUtil`.
//!
//! Java's `Class<?>`-keyed maps are replaced by small Rust enums since Rust
//! has no runtime class objects (SPEC §4).

/// Well-known member name: Java `BasicUtil.LENGTH`.
pub const LENGTH: &str = "length";

/// Well-known member name: Java `BasicUtil.CLASS`.
pub const CLASS: &str = "class";

/// `NumKind` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/BasicUtil.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Numeric kinds used for promotion, mirroring the Java classes handled by
/// `BasicUtil.numberPromoteLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.utils.BasicUtil。
pub enum NumKind {
    /// Java `Byte` 类型或语义类别。
    Byte,
    /// Java `Short` 类型或语义类别。
    Short,
    /// Java `Int` 类型或语义类别。
    Int,
    /// Java `Long` 类型或语义类别。
    Long,
    /// Java `BigInteger` 类型或语义类别。
    BigInteger,
    /// Java `Float` 类型或语义类别。
    Float,
    /// Java `Double` 类型或语义类别。
    Double,
    /// Java `BigDecimal` 类型或语义类别。
    BigDecimal,
}

impl NumKind {
    /// 处理 promote level 对应的领域职责。
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

/// `PrimitiveType` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/BasicUtil.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java primitive types plus their boxed forms, mirroring the keys/values of
/// `BasicUtil.primitiveMap`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.utils.BasicUtil。
pub enum PrimitiveType {
    /// Java `Boolean` 类型或语义类别。
    Boolean,
    /// Java `Character` 类型或语义类别。
    Character,
    /// Java `Double` 类型或语义类别。
    Double,
    /// Java `Float` 类型或语义类别。
    Float,
    /// Java `Int` 类型或语义类别。
    Int,
    /// Java `Long` 类型或语义类别。
    Long,
    /// Java `Byte` 类型或语义类别。
    Byte,
    /// Java `Short` 类型或语义类别。
    Short,
}

/// 处理 trans to primitive 对应的领域职责。
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
}
