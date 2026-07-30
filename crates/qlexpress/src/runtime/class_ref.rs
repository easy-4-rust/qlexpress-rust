//! 类型引用,对应 Java 中 `Class<?>` 在 QLExpress 运行期的角色
//! (Rust 新增物,承载 `MetaClass.getClz()` 与成员解析中的类型语义)。

use crate::runtime::data::convert::obj_type_convertor::TargetType;

/// 类型引用,对应 Java 侧的 `Class<?>`(Rust 适配类型,Java 无同名类)。
///
/// Java 版用 `Class` 对象描述类型(构造器/方法签名、`MetaClass` 负载等);
/// Rust 按 SPEC §4 用「原语转换目标 + 注册类型名」二分表达:
/// - `Primitive`: Java 的原语/包装类,对应一个 [`TargetType`] 转换目标;
/// - `Named`: 具名类型(内建类型或宿主注册类型),以 Java 全限定名标识。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClassRef {
    /// 原语/包装转换目标(Java 的 `int.class`/`Integer.class` 等)。
    Primitive(TargetType),
    /// 具名(已注册或宿主)类型,如 `java.lang.String`。
    Named(String),
}

impl From<TargetType> for ClassRef {
    fn from(value: TargetType) -> Self {
        ClassRef::Primitive(value)
    }
}

impl ClassRef {
    /// 对应 Java 方法 `Class.getName()`。
    pub fn java_name(&self) -> &str {
        match self {
            ClassRef::Primitive(target) => target.java_name(),
            ClassRef::Named(name) => name.as_str(),
        }
    }

    /// 对应 Java 方法 `Class.getSimpleName()`。
    pub fn simple_name(&self) -> &str {
        let name = self.java_name();
        name.rsplit('.').next().unwrap_or(name)
    }

    /// 按 Java 风格类型名解析(编译期类字面量使用)。
    /// 原语名映射为转换目标,对应 Java 侧 `int`/`Integer` 等 `Class` 常量。
    pub fn from_name(name: &str) -> ClassRef {
        let primitive = match name {
            "boolean" | "java.lang.Boolean" => Some(TargetType::Boolean),
            "byte" | "java.lang.Byte" => Some(TargetType::Byte),
            "short" | "java.lang.Short" => Some(TargetType::Short),
            "int" | "java.lang.Integer" => Some(TargetType::Int),
            "long" | "java.lang.Long" => Some(TargetType::Long),
            "float" | "java.lang.Float" => Some(TargetType::Float),
            "double" | "java.lang.Double" => Some(TargetType::Double),
            "java.math.BigInteger" => Some(TargetType::BigInteger),
            "java.math.BigDecimal" => Some(TargetType::BigDecimal),
            "char" | "java.lang.Character" => Some(TargetType::Character),
            _ => None,
        };
        match primitive {
            Some(target) => ClassRef::Primitive(target),
            None => ClassRef::Named(name.to_string()),
        }
    }

    /// 转为参数转换目标(Java `ParametersTypeConvertor.cast` 的 `Class<?>[]`
    /// 元素)。具名类型在 Java 中是任意 `Class`,对应转换目标的 `Any`
    /// (接受任意值,不转换)。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn to_target_type(&self) -> TargetType {
        match self {
            ClassRef::Primitive(target) => *target,
            ClassRef::Named(_) => TargetType::Any,
        }
    }

    /// 对应 Java `clz == Object.class` 判断。
    pub fn is_java_object(&self) -> bool {
        matches!(self, ClassRef::Named(name) if name == "java.lang.Object")
    }
}
