//! Basic helpers, mirroring Java `BasicUtil`.
//!
//! Java's `Class<?>`-keyed maps are replaced by small Rust enums since Rust
//! has no runtime class objects (SPEC §4).

/// Well-known member name: Java `BasicUtil.LENGTH`.
pub const LENGTH: &str = "length";

/// Well-known member name: Java `BasicUtil.CLASS`.
pub const CLASS: &str = "class";

/// Numeric kinds used for promotion, mirroring the Java classes handled by
/// `BasicUtil.numberPromoteLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumKind {
    Byte,
    Short,
    Int,
    Long,
    BigInteger,
    Float,
    Double,
    BigDecimal,
}

impl NumKind {
    /// Java `BasicUtil.numberPromoteLevel`:
    /// byte=0, short=1, int=2, long=3, BigInteger=4, float=5, double=6,
    /// BigDecimal=7.
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

/// Java primitive types plus their boxed forms, mirroring the keys/values of
/// `BasicUtil.primitiveMap`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Boolean,
    Character,
    Double,
    Float,
    Int,
    Long,
    Byte,
    Short,
}

/// Java `BasicUtil.transToPrimitive`: in Rust boxed and primitive forms are
/// the same type, so this is the identity mapping kept for API parity.
pub fn trans_to_primitive(primitive: PrimitiveType) -> PrimitiveType {
    primitive
}

/// 生成 Java Bean getter/setter 方法名。
/// 对应 Java: `com.alibaba.qlexpress4.utils.BasicUtil`。
pub struct BasicUtil;

impl BasicUtil {
    /// Java `BasicUtil.getGetter`: `"get" + Capitalized(s)`.
    pub fn get_getter(s: &str) -> String {
        capitalize_prefixed("get", s)
    }

    /// Java `BasicUtil.getSetter`: `"set" + Capitalized(s)`.
    pub fn get_setter(s: &str) -> String {
        capitalize_prefixed("set", s)
    }

    /// Java `BasicUtil.getIsGetter`: `"is" + Capitalized(s)`.
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
