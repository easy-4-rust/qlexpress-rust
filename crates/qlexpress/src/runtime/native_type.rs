//! 注册类型描述,对应 Java 中一个 `Class` 的反射能力面(SPEC §4 `NativeType`;
//! Rust 新增物,替代 `Class.getMethods()/getFields()/getConstructors()`)。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub use super::native_constructor_candidate::NativeConstructorCandidate;
pub use super::native_method_candidate::NativeMethodCandidate;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::value::DataValue;

/// 原生(宿主/内建)方法:`fn(接收者, 参数) -> 结果`。
/// 对应 Java `java.lang.reflect.Method` 的可调用形态。
pub type NativeMethod = Rc<dyn Fn(&DataValue, &[DataValue]) -> Result<DataValue, QLException>>;

/// 原生构造器:`fn(参数) -> 实例`。对应 Java `java.lang.reflect.Constructor`。
pub type NativeConstructor = Rc<dyn Fn(&[DataValue]) -> Result<DataValue, QLException>>;

/// 原生字段读取器。对应 Java `java.lang.reflect.Field` 的读操作(含 getter)。
pub type NativeFieldGetter = Rc<dyn Fn(&DataValue) -> Option<DataValue>>;

/// 原生字段写入器。返回 `true` 表示值类型可转换且写入成功。
/// 对应 Java `Field#set`/setter 方法形成的 `FieldValue` 写通道。
pub type NativeFieldSetter = Rc<dyn Fn(&DataValue, &DataValue) -> bool>;

/// 可写静态字段存储。对应 Java 非 `final static Field` 的共享可变值。
pub type NativeStaticField = Rc<RefCell<DataValue>>;

impl NativeMethodCandidate {
    /// 创建一个显式签名的方法候选。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn new(parameter_types: Vec<ClassRef>, var_args: bool, method: NativeMethod) -> Self {
        Self {
            parameter_types,
            var_args,
            method,
        }
    }
}

impl NativeConstructorCandidate {
    /// 创建一个显式签名的构造器候选。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn new(
        parameter_types: Vec<ClassRef>,
        var_args: bool,
        constructor: NativeConstructor,
    ) -> Self {
        Self {
            parameter_types,
            var_args,
            constructor,
        }
    }
}

/// 已注册类型,对应 Java 一个 `Class` 暴露给脚本的反射面(SPEC §4)。
///
/// Java 版靠反射在调用现场解析重载;Rust 版重载按名注册(同名多实现时
/// 由注册的闭包内部按参数分派,对应 Java `MemberResolver` 的职责内移)。
#[derive(Default)]
pub struct NativeType {
    /// 规范类型名(Java `Class.getName()`)。
    pub name: String,
    /// 是否为 Java 接口。Rust 显式元数据，对应 `Class#isInterface()`。
    pub is_interface: bool,
    /// 接口的抽象方法名列表。对应
    /// `Class#getMethods()` 中 `Modifier.isAbstract(...)` 的方法集合。
    pub abstract_methods: Vec<String>,
    /// `NewInstanceInstruction` 使用的构造器(Java `Constructor`)。
    pub constructor: Option<NativeConstructor>,
    /// 多构造器候选；非空时在调用现场按 Java 匹配优先级选择。
    pub constructor_candidates: Vec<NativeConstructorCandidate>,
    /// 实例方法表(按名;Java 实例 `Method` 集合)。
    pub methods: HashMap<String, NativeMethod>,
    /// 同名实例方法候选表。
    pub method_candidates: HashMap<String, Vec<NativeMethodCandidate>>,
    /// 静态方法表(按名;Java 静态 `Method` 集合)。
    pub static_methods: HashMap<String, NativeMethod>,
    /// 同名静态方法候选表。
    pub static_method_candidates: HashMap<String, Vec<NativeMethodCandidate>>,
    /// 实例字段读取器(按名;Java `Field`/getter 方法)。
    pub fields: HashMap<String, NativeFieldGetter>,
    /// 实例字段写入器；不存在表示该字段只读。
    pub field_setters: HashMap<String, NativeFieldSetter>,
    /// 静态字段值(按名;Java 静态 `Field`)。
    /// 此表用于常量/只读静态字段。
    pub static_fields: HashMap<String, DataValue>,
    /// 可写静态字段；读取时返回 `LeftValue`，赋值会写回共享单元。
    pub static_field_cells: HashMap<String, NativeStaticField>,
    /// 字段别名表:字段名 -> 别名列表,对应 Java 字段上的 `@QLAlias` 注解
    /// (Rust 无运行时注解,按 SPEC §4 显式注册)。
    pub field_aliases: HashMap<String, Vec<String>>,
    /// 方法别名表:方法名 -> 别名列表,对应 Java 方法上的 `@QLAlias` 注解。
    pub method_aliases: HashMap<String, Vec<String>>,
    /// 直接父类/接口名称。候选方法解析和引用可赋值检查会按顺序递归。
    pub supertypes: Vec<String>,
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

    /// 创建显式接口类型并登记其抽象方法。
    ///
    /// 对应 Java `Class#isInterface()` 与 `Class#getMethods()` 的 Rust
    /// 注册形态；恰有一个抽象方法时，该类型可作为 Lambda/SAM 形参参与
    /// [`crate::runtime::member_resolver::MemberResolver`] 重载选择。
    ///
    /// # 参数
    ///
    /// - `name`：Java 规范接口名。
    /// - `abstract_methods`：包含继承所得方法在内的抽象方法名。
    pub fn interface<I, S>(name: impl Into<String>, abstract_methods: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        NativeType {
            name: name.into(),
            is_interface: true,
            abstract_methods: abstract_methods.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }

    /// 注册一个同名实例方法候选。
    /// 对应 Java：`Class#getMethods()` 中同名 `Method` 候选的收集。
    pub fn add_method_candidate(
        &mut self,
        name: impl Into<String>,
        candidate: NativeMethodCandidate,
    ) {
        self.method_candidates
            .entry(name.into())
            .or_default()
            .push(candidate);
    }

    /// 注册一个同名静态方法候选。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn add_static_method_candidate(
        &mut self,
        name: impl Into<String>,
        candidate: NativeMethodCandidate,
    ) {
        self.static_method_candidates
            .entry(name.into())
            .or_default()
            .push(candidate);
    }

    /// 注册一个构造器候选。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn add_constructor_candidate(&mut self, candidate: NativeConstructorCandidate) {
        self.constructor_candidates.push(candidate);
    }
}
