//! 类型引用,对应 Java 中 `Class<?>` 在 QLExpress 运行期的角色
//! (Rust 新增物,承载 `MetaClass.getClz()` 与成员解析中的类型语义)。

use crate::runtime::data::convert::obj_type_convertor::TargetType;
use std::hash::{Hash, Hasher};

/// 类型引用,对应 Java 侧的 `Class<?>`(Rust 适配类型,Java 无同名类)。
///
/// Java 版用 `Class` 对象描述类型(构造器/方法签名、`MetaClass` 负载等);
/// Rust 按 SPEC §4 显式区分 Java 原语、包装/数值引用类型和其他具名类型:
/// - `Primitive`: Java 的真实原语 `Class`，如 `int.class`;
/// - `Boxed`: Java 包装类及 `BigInteger`/`BigDecimal`;
/// - `Named`: 具名类型(内建类型或宿主注册类型),以 Java 全限定名标识。
#[derive(Clone, Debug)]
pub enum ClassRef {
    /// Java 原语类型，如 `int.class`。
    Primitive(TargetType),
    /// Java 包装/数值引用类型，如 `Integer.class`、`BigDecimal.class`。
    Boxed(TargetType),
    /// 具名(已注册或宿主)类型,如 `java.lang.String`。
    Named(String),
}

impl PartialEq for ClassRef {
    fn eq(&self, other: &Self) -> bool {
        self.java_name() == other.java_name()
    }
}

impl Eq for ClassRef {}

impl Hash for ClassRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.java_name().hash(state);
    }
}

impl From<TargetType> for ClassRef {
    fn from(value: TargetType) -> Self {
        match value {
            TargetType::Any => ClassRef::Named("java.lang.Object".to_string()),
            target => ClassRef::Boxed(target),
        }
    }
}

impl ClassRef {
    /// 对应 Java 方法 `Class.getName()`。
    pub fn java_name(&self) -> &str {
        match self {
            ClassRef::Primitive(target) => primitive_java_name(*target),
            ClassRef::Boxed(target) => target.java_name(),
            ClassRef::Named(name) => name.as_str(),
        }
    }

    /// 对应 Java 方法 `Class.getSimpleName()`。
    pub fn simple_name(&self) -> String {
        if let Some(component) = self.component_type() {
            return format!("{}[]", component.simple_name());
        }
        let name = self.java_name();
        name.rsplit('.').next().unwrap_or(name).to_string()
    }

    /// 按 Java 风格类型名解析(编译期类字面量使用)。
    /// 原语名与包装类名保持不同 `Class` 身份，供 Java 重载解析准确执行
    /// `EQUAL`、`UNBOX` 与数值提升优先级。
    pub fn from_name(name: &str) -> ClassRef {
        if name.starts_with('[') {
            return ClassRef::Named(name.to_string());
        }
        if let Some(component_name) = name.strip_suffix("[]") {
            return ClassRef::array_of(ClassRef::from_name(component_name));
        }
        match name {
            "boolean" => ClassRef::Primitive(TargetType::Boolean),
            "byte" => ClassRef::Primitive(TargetType::Byte),
            "short" => ClassRef::Primitive(TargetType::Short),
            "int" => ClassRef::Primitive(TargetType::Int),
            "long" => ClassRef::Primitive(TargetType::Long),
            "float" => ClassRef::Primitive(TargetType::Float),
            "double" => ClassRef::Primitive(TargetType::Double),
            "char" => ClassRef::Primitive(TargetType::Character),
            "java.lang.Boolean" => ClassRef::Boxed(TargetType::Boolean),
            "java.lang.Byte" => ClassRef::Boxed(TargetType::Byte),
            "java.lang.Short" => ClassRef::Boxed(TargetType::Short),
            "java.lang.Integer" => ClassRef::Boxed(TargetType::Int),
            "java.lang.Long" => ClassRef::Boxed(TargetType::Long),
            "java.lang.Float" => ClassRef::Boxed(TargetType::Float),
            "java.lang.Double" => ClassRef::Boxed(TargetType::Double),
            "java.math.BigInteger" => ClassRef::Boxed(TargetType::BigInteger),
            "java.math.BigDecimal" => ClassRef::Boxed(TargetType::BigDecimal),
            "java.lang.Character" => ClassRef::Boxed(TargetType::Character),
            _ => ClassRef::Named(name.to_string()),
        }
    }

    /// 转为参数转换目标(Java `ParametersTypeConvertor.cast` 的 `Class<?>[]`
    /// 元素)。具名类型在 Java 中是任意 `Class`,对应转换目标的 `Any`
    /// (接受任意值,不转换)。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn to_target_type(&self) -> TargetType {
        match self {
            ClassRef::Primitive(target) | ClassRef::Boxed(target) => *target,
            ClassRef::Named(_) => TargetType::Any,
        }
    }

    /// 对应 Java `clz == Object.class` 判断。
    pub fn is_java_object(&self) -> bool {
        matches!(self, ClassRef::Named(name) if name == "java.lang.Object")
    }

    /// 创建 Java 数组类引用，对应 `Array.newInstance(component, 0).getClass()`。
    pub fn array_of(component: ClassRef) -> ClassRef {
        let component_name = component.java_name();
        let array_name = if component_name.starts_with('[') {
            format!("[{component_name}")
        } else {
            match component_name {
                "boolean" => "[Z".to_string(),
                "byte" => "[B".to_string(),
                "char" => "[C".to_string(),
                "short" => "[S".to_string(),
                "int" => "[I".to_string(),
                "long" => "[J".to_string(),
                "float" => "[F".to_string(),
                "double" => "[D".to_string(),
                _ => format!("[L{component_name};"),
            }
        };
        ClassRef::Named(array_name)
    }

    /// 返回 Java 数组组件类型，对应 `Class#getComponentType()`。
    ///
    /// 非数组类型返回 `None`；多维数组每次只剥离一层。
    pub fn component_type(&self) -> Option<ClassRef> {
        let name = self.java_name();
        if let Some(descriptor) = name.strip_prefix('[') {
            return match descriptor {
                "Z" => Some(ClassRef::Primitive(TargetType::Boolean)),
                "B" => Some(ClassRef::Primitive(TargetType::Byte)),
                "C" => Some(ClassRef::Primitive(TargetType::Character)),
                "S" => Some(ClassRef::Primitive(TargetType::Short)),
                "I" => Some(ClassRef::Primitive(TargetType::Int)),
                "J" => Some(ClassRef::Primitive(TargetType::Long)),
                "F" => Some(ClassRef::Primitive(TargetType::Float)),
                "D" => Some(ClassRef::Primitive(TargetType::Double)),
                nested if nested.starts_with('[') => Some(ClassRef::Named(nested.to_string())),
                reference if reference.starts_with('L') && reference.ends_with(';') => Some(
                    ClassRef::from_name(&reference[1..reference.len().saturating_sub(1)]),
                ),
                _ => None,
            };
        }
        // 兼容迁移早期缓存与宿主 API 传入的规范展示形式。
        name.strip_suffix("[]").map(ClassRef::from_name)
    }
}

/// 返回 Java 原语 `Class#getName()`，不得复用包装类规范名。
fn primitive_java_name(target: TargetType) -> &'static str {
    match target {
        TargetType::Boolean => "boolean",
        TargetType::Byte => "byte",
        TargetType::Short => "short",
        TargetType::Int => "int",
        TargetType::Long => "long",
        TargetType::Float => "float",
        TargetType::Double => "double",
        TargetType::Character => "char",
        // 这些类型不存在 Java 原语形态；调用方不应构造该组合。保留规范名
        // 以避免手工构造无效值时破坏诊断和序列化。
        TargetType::BigInteger | TargetType::BigDecimal | TargetType::Any => target.java_name(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: Java `int.class != Integer.class`，但两者共享同一转换目标。
    #[test]
    fn primitive_and_boxed_classes_keep_distinct_identity() {
        let primitive = ClassRef::from_name("int");
        let boxed = ClassRef::from_name("java.lang.Integer");
        assert_eq!(primitive.java_name(), "int");
        assert_eq!(boxed.java_name(), "java.lang.Integer");
        assert_ne!(primitive, boxed);
        assert_eq!(primitive.to_target_type(), TargetType::Int);
        assert_eq!(boxed.to_target_type(), TargetType::Int);
    }

    /// SOURCE_PARITY: Java varargs 和多维数组每次只读取一层组件类型。
    #[test]
    fn array_component_type_preserves_primitive_and_boxed_class() {
        let primitive_array = ClassRef::array_of(ClassRef::from_name("int"));
        let boxed_array = ClassRef::array_of(ClassRef::from_name("java.lang.Integer"));
        assert_eq!(primitive_array.java_name(), "[I");
        assert_eq!(
            primitive_array.component_type(),
            Some(ClassRef::Primitive(TargetType::Int))
        );
        assert_eq!(boxed_array.java_name(), "[Ljava.lang.Integer;");
        assert_eq!(
            boxed_array.component_type(),
            Some(ClassRef::Boxed(TargetType::Int))
        );
    }
}
