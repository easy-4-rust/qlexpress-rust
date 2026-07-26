//! 注册类型描述,对应 Java 中一个 `Class` 的反射能力面(SPEC §4 `NativeType`;
//! Rust 新增物,替代 `Class.getMethods()/getFields()/getConstructors()`)。

use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::value::DataValue;

/// 原生(宿主/内建)方法:`fn(接收者, 参数) -> 结果`。
/// 对应 Java `java.lang.reflect.Method` 的可调用形态。
pub type NativeMethod = Rc<dyn Fn(&DataValue, &[DataValue]) -> Result<DataValue, QLException>>;

/// 原生构造器:`fn(参数) -> 实例`。对应 Java `java.lang.reflect.Constructor`。
pub type NativeConstructor = Rc<dyn Fn(&[DataValue]) -> Result<DataValue, QLException>>;

/// 原生字段读取器。对应 Java `java.lang.reflect.Field` 的读操作(含 getter)。
pub type NativeFieldGetter = Rc<dyn Fn(&DataValue) -> Option<DataValue>>;

/// 已注册类型,对应 Java 一个 `Class` 暴露给脚本的反射面(SPEC §4)。
///
/// Java 版靠反射在调用现场解析重载;Rust 版重载按名注册(同名多实现时
/// 由注册的闭包内部按参数分派,对应 Java `MemberResolver` 的职责内移)。
#[derive(Default)]
pub struct NativeType {
    /// 规范类型名(Java `Class.getName()`)。
    pub name: String,
    /// `NewInstanceInstruction` 使用的构造器(Java `Constructor`)。
    pub constructor: Option<NativeConstructor>,
    /// 实例方法表(按名;Java 实例 `Method` 集合)。
    pub methods: HashMap<String, NativeMethod>,
    /// 静态方法表(按名;Java 静态 `Method` 集合)。
    pub static_methods: HashMap<String, NativeMethod>,
    /// 实例字段读取器(按名;Java `Field`/getter 方法)。
    pub fields: HashMap<String, NativeFieldGetter>,
    /// 静态字段值(按名;Java 静态 `Field`)。
    pub static_fields: HashMap<String, DataValue>,
    /// 字段别名表:字段名 -> 别名列表,对应 Java 字段上的 `@QLAlias` 注解
    /// (Rust 无运行时注解,按 SPEC §4 显式注册)。
    pub field_aliases: HashMap<String, Vec<String>>,
    /// 方法别名表:方法名 -> 别名列表,对应 Java 方法上的 `@QLAlias` 注解。
    pub method_aliases: HashMap<String, Vec<String>>,
}

impl NativeType {
    /// 以规范名创建空类型描述。对应 Java `Class.forName(name)` 得到的
    /// 「只有名字」的类型句柄。
    pub fn named(name: impl Into<String>) -> Self {
        NativeType {
            name: name.into(),
            ..Default::default()
        }
    }
}
