//! 对应 Java 类：com.alibaba.qlexpress4.aparser.BuiltInTypesSet
//!
//! Java 中 `BuiltInTypesSet` 是一个常量集合 + 静态方法 `getCls(String)`：
//! 持有 8 个 Java 原始类型包装类（`Byte/Short/Integer/Long/Float/Double/Boolean/Character`）
//! 的字面量名（"byte"/"short"/"int"/.../"char"），解析期内 `parseDeclType`/`parseClsIds`
//! 通过 `getCls(lexeme)` 判断当前 token 是否为内建类型。
//!
//! 与 Java 实现的差异：
//! - Rust 无 `Class<?>`，类型以 [`crate::runtime::class_ref::ClassRef`] 表达；
//! - 8 个字面量常量在 `aparser/token.rs` 中作为词法 token ID（`BYTE`/`SHORT`/...）
//!   被词法分析器直接消费；本文件作为"类身份"对照占位保留（SPEC §2 一文件一对象）；
//! - `get_cls(lexeme)` 返回 [`BuiltInType`] 枚举，便于上层判定类型种类。

/// Java `BuiltInTypesSet.BYTE` 等常量的字面量文本。
pub const BYTE: &str = "byte";
/// Java `BuiltInTypesSet.SHORT`。
pub const SHORT: &str = "short";
/// Java `BuiltInTypesSet.INT`。
pub const INT: &str = "int";
/// Java `BuiltInTypesSet.LONG`。
pub const LONG: &str = "long";
/// Java `BuiltInTypesSet.FLOAT`。
pub const FLOAT: &str = "float";
/// Java `BuiltInTypesSet.DOUBLE`。
pub const DOUBLE: &str = "double";
/// Java `BuiltInTypesSet.BOOLEAN`。
pub const BOOLEAN: &str = "boolean";
/// Java `BuiltInTypesSet.CHAR`。
pub const CHAR: &str = "char";

/// 8 个 Java 原始类型包装类（对应 Java `Byte.class`/`Integer.class`/...）的
/// Rust 端分类枚举。
///
/// 与 Java `Class<?>` 的对应：
/// - `Byte` ↔ Rust `i8`
/// - `Short` ↔ Rust `i16`
/// - `Int` ↔ Rust `i32`（`DataValue::Int`）
/// - `Long` ↔ Rust `i64`（`DataValue::Long`）
/// - `Float` ↔ Rust `f32`
/// - `Double` ↔ Rust `f64`（`DataValue::Double`）
/// - `Boolean` ↔ Rust `bool`（`DataValue::Bool`）
/// - `Char` ↔ Java UTF-16 code unit `u16`（`DataValue::Char`）
///
/// 对应 Java: com.alibaba.qlexpress4.aparser.BuiltInTypesSet。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltInType {
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
    /// Java `Boolean` 类型或语义类别。
    Boolean,
    /// Java `Char` 类型或语义类别。
    Char,
}

impl BuiltInType {
    /// 返回对应 Java 字面量（`"byte"`/`"int"`/...）。
    pub fn lexeme(self) -> &'static str {
        match self {
            BuiltInType::Byte => BYTE,
            BuiltInType::Short => SHORT,
            BuiltInType::Int => INT,
            BuiltInType::Long => LONG,
            BuiltInType::Float => FLOAT,
            BuiltInType::Double => DOUBLE,
            BuiltInType::Boolean => BOOLEAN,
            BuiltInType::Char => CHAR,
        }
    }

    /// 返回对应 Java `Class<?>` 全限定名（与 Java `Byte.class.getName()` 一致）。
    pub fn java_class_name(self) -> &'static str {
        match self {
            BuiltInType::Byte => "java.lang.Byte",
            BuiltInType::Short => "java.lang.Short",
            BuiltInType::Int => "java.lang.Integer",
            BuiltInType::Long => "java.lang.Long",
            BuiltInType::Float => "java.lang.Float",
            BuiltInType::Double => "java.lang.Double",
            BuiltInType::Boolean => "java.lang.Boolean",
            BuiltInType::Char => "java.lang.Character",
        }
    }
}

/// Java `BuiltInTypesSet.getCls(String lexeme)`。
///
/// 与 Java 一致：传入字面量文本，命中 8 个内建类型之一则返回 `Some`，
/// 否则返回 `None`（Java 返回 `null`）。
/// 对应 Java: com.alibaba.qlexpress4.aparser.BuiltInTypesSet#getCls。
pub fn get_cls(lexeme: &str) -> Option<BuiltInType> {
    match lexeme {
        BYTE => Some(BuiltInType::Byte),
        SHORT => Some(BuiltInType::Short),
        INT => Some(BuiltInType::Int),
        LONG => Some(BuiltInType::Long),
        FLOAT => Some(BuiltInType::Float),
        DOUBLE => Some(BuiltInType::Double),
        BOOLEAN => Some(BuiltInType::Boolean),
        CHAR => Some(BuiltInType::Char),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_cls_matches_all_eight_java_types() {
        assert_eq!(get_cls("byte"), Some(BuiltInType::Byte));
        assert_eq!(get_cls("short"), Some(BuiltInType::Short));
        assert_eq!(get_cls("int"), Some(BuiltInType::Int));
        assert_eq!(get_cls("long"), Some(BuiltInType::Long));
        assert_eq!(get_cls("float"), Some(BuiltInType::Float));
        assert_eq!(get_cls("double"), Some(BuiltInType::Double));
        assert_eq!(get_cls("boolean"), Some(BuiltInType::Boolean));
        assert_eq!(get_cls("char"), Some(BuiltInType::Char));
    }

    #[test]
    fn get_cls_returns_none_for_non_builtin() {
        assert_eq!(get_cls("String"), None);
        assert_eq!(get_cls(""), None);
        assert_eq!(get_cls("bigint"), None);
    }

    #[test]
    fn lexeme_round_trip() {
        for original in [BYTE, SHORT, INT, LONG, FLOAT, DOUBLE, BOOLEAN, CHAR] {
            let ty = get_cls(original).expect("builtin");
            assert_eq!(ty.lexeme(), original);
        }
    }

    #[test]
    fn java_class_name_aligns_with_jvm_boxed_types() {
        assert_eq!(BuiltInType::Int.java_class_name(), "java.lang.Integer");
        assert_eq!(BuiltInType::Char.java_class_name(), "java.lang.Character");
    }
}
