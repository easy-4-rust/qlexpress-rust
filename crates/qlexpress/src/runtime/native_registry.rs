//! 显式类型注册表,替代 Java 反射(SPEC §4)。
//! 对应 Java `com.alibaba.qlexpress4.runtime.ReflectLoader` 的
//! `loadConstructor`/`loadField`/`loadMethod` 职责,以及
//! `ClassSupplier`/`DefaultClassSupplier` 的类型供给职责(Rust 新增物,
//! Java 无同名类;内建方法子集对齐 Java 版脚本中 String/List/Map/数值
//! 的常用方法)。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::member::field_handler::Preferred as PreferredFieldHandler;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::convert::number_compare;
use crate::runtime::data::convert::parameters_type_convertor::ParametersTypeConvertor;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::{FieldValue, MapItemValue};
use crate::runtime::function::ExtensionFunction;
use crate::runtime::java_collector::JavaCollector;
use crate::runtime::java_map_entry::JavaMapEntry;
use crate::runtime::java_stream::JavaStream;
use crate::runtime::member_resolver::MemberResolver;
use crate::runtime::meta_class::{as_meta_class, MetaClass};
use crate::runtime::native_type::{
    NativeConstructor, NativeConstructorCandidate, NativeMethod, NativeMethodCandidate, NativeType,
};
use crate::runtime::opaque_native_object::OpaqueNativeObject;
use crate::runtime::qvm_runtime::current_time_millis;
use crate::runtime::value::{DataValue, QValue};
use crate::security::ql_security_strategy::{NativeMember, QLSecurityStrategy};
use crate::utils::basic_util;
use crate::utils::cache_util::CacheUtil;

/// 显式类型注册表。对应 Java: `ReflectLoader`(按类型加载构造器/字段/方法)
/// + `DefaultClassSupplier`(类型白名单式供给)。
///
/// Java 语义要点:Java `loadField`/`loadMethod` 通过反射现场解析成员,
/// 并对数组/List 的 `length`、Map 的 key、类的静态成员有特殊分支;
/// Rust 版逐一复现这些特殊分支,普通成员走注册表查询。
///
/// 安全策略接线点(Java `ReflectLoader.check(Member)`):
/// Java 在反射取得成员后过 `securityStrategy.check(member)`,不通过则按
/// 「成员不存在」(`null`)处理;Rust 在**注册类型成员**的解析点做同样
/// 判定(见 [`NativeRegistry::resolve_method`] / [`NativeRegistry::load_field`])。
/// 偏差:内建类型(String/List/Map/数值)的方法子集是 Rust 语言内核的
/// 一部分,不过策略(Java 中它们也走反射,`isolation` 默认下同样被拦);
/// 注册表自身默认策略为 `open`(Java `InitOptions` 默认 `isolation`,
/// 由 `Express4Runner` 构造时显式接线,见 `set_security_strategy`)。
/// 对应 Java: 无（Rust 原生适配）。
pub struct NativeRegistry {
    /// 类型名 -> 注册类型(Java 侧为 `ClassLoader` 可加载的所有类)。
    types: HashMap<String, NativeType>,
    /// 扩展函数表。Java `ReflectLoader.loadMethod` 在隔离策略判断之前解析
    /// `ExtensionFunction`，因此它必须与受安全策略约束的反射方法分开。
    extension_methods: HashMap<(String, String), NativeMethod>,
    /// 成员访问安全策略(Java `ReflectLoader.securityStrategy`)。
    /// `RefCell`:注册表经 `Rc` 共享给 QVM,策略需在 runner 层可改。
    security_strategy: RefCell<QLSecurityStrategy>,
    /// 函数式接口判定缓存。对应 Java `CacheUtil` 的 Class 级缓存；
    /// 每个注册表独立，避免租户/宿主模型之间的同名类型污染。
    function_interface_cache: CacheUtil,
}

impl NativeRegistry {
    /// 空注册表。对应 Java `new ReflectLoader()`(无任何已知类型)。
    ///
    /// # Returns
    ///
    /// 返回默认开放成员策略且不包含任何类型或扩展方法的注册表。
    pub fn new() -> Self {
        NativeRegistry {
            types: HashMap::new(),
            extension_methods: HashMap::new(),
            // 注册表裸用时默认放行;Express4Runner 构造时按
            // `InitOptions.securityStrategy` 覆盖(Java 默认 `isolation`)。
            security_strategy: RefCell::new(QLSecurityStrategy::open()),
            function_interface_cache: CacheUtil::new(),
        }
    }

    /// 设置成员访问安全策略。对应 Java `ReflectLoader` 持有的
    /// `securityStrategy`(由 `InitOptions` 注入)。
    ///
    /// # Arguments
    ///
    /// * `security_strategy` - 后续构造器、字段和方法解析共同使用的策略。
    pub fn set_security_strategy(&self, security_strategy: QLSecurityStrategy) {
        *self.security_strategy.borrow_mut() = security_strategy;
    }

    /// 当前安全策略。对应 Java `InitOptions.getSecurityStrategy()` 经
    /// `ReflectLoader` 读取的值。
    ///
    /// # Returns
    ///
    /// 返回当前策略快照。
    pub fn security_strategy(&self) -> QLSecurityStrategy {
        self.security_strategy.borrow().clone()
    }

    /// Java `ReflectLoader.check(Member)`:`check` 不通过即视为成员不存在。
    fn check_member(&self, type_name: &str, member_name: &str) -> bool {
        self.security_strategy
            .borrow()
            .check(&NativeMember::new(type_name, member_name))
    }

    /// 判断原生成员是否被当前安全策略允许。
    ///
    /// 对应 Java 私有方法 `ReflectLoader#securityFilter(Member)`；供
    /// `NativeObject` 动态分派在调用前执行与反射成员一致的检查。
    ///
    /// # Arguments
    ///
    /// * `type_name` - 成员声明类型的 Java 规范名。
    /// * `member_name` - 构造器、字段或方法名。
    ///
    /// # Returns
    ///
    /// 当前安全策略允许访问该成员时返回 `true`。
    pub fn is_member_allowed(&self, type_name: &str, member_name: &str) -> bool {
        self.check_member(type_name, member_name)
    }
}

impl Default for NativeRegistry {
    /// 与 [`NativeRegistry::new`] 一致(默认放行策略)。
    fn default() -> Self {
        NativeRegistry::new()
    }
}

impl NativeRegistry {
    /// 预置内建类型的注册表(SPEC §4:String/List/Map/数值 常用方法子集)。
    /// 对应 Java: 无（Rust 原生适配）。
    ///
    /// # Returns
    ///
    /// 返回已注册 QLExpress 内建 Java 类型锚点的注册表。
    pub fn with_builtins() -> Self {
        let mut registry = NativeRegistry::new();
        registry.register_builtin_types();
        registry
    }

    /// 注册类型。对应 Java `ClassSupplier.addClass` 一类的类型供给。
    ///
    /// # Arguments
    ///
    /// * `native_type` - 包含规范类型名及显式构造器、字段和方法的描述。
    pub fn register_type(&mut self, native_type: NativeType) {
        self.types.insert(native_type.name.clone(), native_type);
    }

    /// 按名取注册类型。对应 Java `Class.forName` 命中已供给类型。
    ///
    /// # Arguments
    ///
    /// * `name` - Java 规范类型名。
    ///
    /// # Returns
    ///
    /// 类型已经显式注册时返回只读描述，否则返回 `None`。
    pub fn get_type(&self, name: &str) -> Option<&NativeType> {
        self.types.get(name)
    }

    /// 为指定类型追加(或覆盖)一个扩展函数。
    ///
    /// 对应 Java 方法 `ReflectLoader#addExtendFunction`；扩展函数在
    /// `StrategyIsolation` 判断之前解析，不属于受反射沙箱约束的成员。
    ///
    /// # Arguments
    ///
    /// * `type_name` - 被扩展类型的 Java 规范名。
    /// * `method_name` - 脚本调用的方法名。
    /// * `method` - 接收目标值和实参数组的宿主实现。
    pub fn register_method(
        &mut self,
        type_name: impl Into<String>,
        method_name: impl Into<String>,
        method: NativeMethod,
    ) {
        self.extension_methods
            .insert((type_name.into(), method_name.into()), method);
    }

    /// 按 Java `@QLAlias` 语义把脚本方法名解析为注册表中的真实方法名。
    ///
    /// Java `MethodHandler` 会先枚举真实方法，再匹配方法上的别名；Rust
    /// 将注解元数据拍平到 `NativeType.method_aliases`，这里同时服务静态
    /// 方法和实例方法分派。
    fn resolve_registered_method_name<'a>(
        native_type: &'a NativeType,
        method_name: &'a str,
        is_static: bool,
    ) -> &'a str {
        let contains_method = |name: &str| {
            if is_static {
                native_type.static_methods.contains_key(name)
                    || native_type.static_method_candidates.contains_key(name)
            } else {
                native_type.methods.contains_key(name)
                    || native_type.method_candidates.contains_key(name)
            }
        };
        if contains_method(method_name) {
            return method_name;
        }
        native_type
            .method_aliases
            .iter()
            .find_map(|(registered_name, aliases)| {
                (contains_method(registered_name)
                    && aliases.iter().any(|alias| alias == method_name))
                .then_some(registered_name.as_str())
            })
            .unwrap_or(method_name)
    }

    // ---- 对应 Java ReflectLoader.loadConstructor ----

    /// 对应 Java 方法 `loadConstructor(Class, Class[])`:取注册构造器;
    /// 参数匹配委托给构造器闭包自身(Java 由 `MemberResolver` 选重载,
    /// Rust 一个类型只注册一个构造入口)。
    ///
    /// # Arguments
    ///
    /// * `clz` - 待实例化的类型引用。
    ///
    /// # Returns
    ///
    /// 安全策略允许且类型注册了兼容构造器时返回调用闭包。
    pub fn load_constructor(&self, clz: &ClassRef) -> Option<NativeConstructor> {
        if !self.check_member(clz.java_name(), "<init>") {
            return None;
        }
        self.types
            .get(clz.java_name())
            .and_then(|native_type| native_type.constructor.as_ref().map(Rc::clone))
    }

    /// 按实参类型选择构造器候选。没有候选元数据时兼容旧的单构造器注册。
    /// 对应 Java: 无（Rust 原生适配）。
    ///
    /// # Arguments
    ///
    /// * `clz` - 待实例化类型。
    /// * `args` - 用于 Java 重载匹配的运行时实参。
    ///
    /// # Returns
    ///
    /// 返回完成必要参数转换的最佳构造器，未授权或无匹配项时返回 `None`。
    pub fn load_constructor_for_args(
        &self,
        clz: &ClassRef,
        args: &[DataValue],
    ) -> Option<NativeConstructor> {
        if !self.check_member(clz.java_name(), "<init>") {
            return None;
        }
        let native_type = self.types.get(clz.java_name())?;
        if let Some(candidate) =
            self.select_constructor_candidate(&native_type.constructor_candidates, args)
        {
            let constructor = Rc::clone(&candidate.constructor);
            let parameter_types = candidate.parameter_types.clone();
            let var_args = candidate.var_args;
            return Some(Rc::new(move |values| {
                let converted = convert_candidate_arguments(values, &parameter_types, var_args);
                constructor(&converted)
            }));
        }
        if native_type.constructor_candidates.is_empty() {
            return native_type.constructor.as_ref().map(Rc::clone);
        }
        None
    }

    // ---- 对应 Java ReflectLoader.loadField ----

    /// 对应 Java 方法 `loadField(Object bean, String fieldName, boolean
    /// skipSecurity, ErrorReporter)`:字段不存在时返回 `None`(Java 返回 `null`)。
    ///
    /// # Arguments
    ///
    /// * `bean` - 字段接收者。
    /// * `field_name` - 字段名、Map 键或内建 `length`/`class` 名称。
    ///
    /// # Returns
    ///
    /// 安全策略允许且字段存在时返回可读或可写 QVM 值。
    pub fn load_field(&self, bean: &DataValue, field_name: &str) -> Option<QValue> {
        self.load_field_with_security(bean, field_name, false)
    }

    /// 加载字段，并按 Java `skipSecurity` 参数决定是否跳过成员策略。
    ///
    /// 对应 Java 方法 `ReflectLoader#loadField(Object, String, boolean,
    /// ErrorReporter)`；脚本指令传 `false`，`Express4Runner#loadField`
    /// 宿主 API 传 `true`。
    ///
    /// # Arguments
    ///
    /// * `bean` - 字段接收者。
    /// * `field_name` - 待解析字段名。
    /// * `skip_security` - 仅宿主 API 可用；为真时跳过成员安全策略。
    ///
    /// # Returns
    ///
    /// 字段可解析时返回对应 QVM 值，否则返回 `None`。
    pub fn load_field_with_security(
        &self,
        bean: &DataValue,
        field_name: &str,
        skip_security: bool,
    ) -> Option<QValue> {
        // Java 通用语义:任何对象都有 `.class`(`obj.getClass()`)。
        // 内建值按 `data_type_name` 还原类引用(原语名经
        // `ClassRef::from_name` 归一到与类字面量 `int` 等一致的
        // Primitive 目标,使 `c.class == int` 之类的比较成立);
        // MetaClass/宿主对象的 `.class` 由下方 Object 分支处理。
        // (对齐测试 cast/cast_express.ql 发现。)
        if field_name == basic_util::CLASS && !matches!(bean, DataValue::Object(_)) {
            let class_ref = ClassRef::from_name(bean.data_type_name());
            return Some(QValue::Data(MetaClass::new(class_ref).into_data_value()));
        }
        match bean {
            // Java 特殊分支:数组 length。
            DataValue::Array(arr) if field_name == basic_util::LENGTH => {
                Some(QValue::Data(DataValue::Int(arr.borrow().len() as i32)))
            }
            // Java 特殊分支:List length。
            DataValue::List(list) if field_name == basic_util::LENGTH => {
                Some(QValue::Data(DataValue::Int(list.borrow().len() as i32)))
            }
            // Java 特殊分支:Map 的字段访问即按 key 取条目(可写左值)。
            DataValue::Map(map) => Some(QValue::Left(Rc::new(RefCell::new(MapItemValue::new(
                Rc::clone(map),
                DataValue::Str(field_name.to_string()),
            ))))),
            DataValue::Object(obj) => {
                // Java 的 MetaClass 分支:`.class` 与静态字段。
                let meta_clz = {
                    let borrowed = obj.borrow();
                    borrowed
                        .as_any()
                        .downcast_ref::<MetaClass>()
                        .map(|meta| meta.clz().clone())
                };
                match meta_clz {
                    Some(clz) => {
                        if field_name == basic_util::CLASS {
                            // Java 返回 Class 对象本身;栈上最接近的值即 MetaClass 数据。
                            return Some(QValue::Data(bean.clone()));
                        }
                        let name = clz.java_name();
                        let native_type = self.types.get(name)?;
                        let registered_name =
                            PreferredFieldHandler::gather_field_recursive(native_type, field_name)?;
                        // 安全策略接线点(Java ReflectLoader.check):
                        // 静态字段访问前过 QLSecurityStrategy。
                        if skip_security || self.check_member(name, &registered_name) {
                            if let Some(cell) = native_type.static_field_cells.get(&registered_name)
                            {
                                let getter_cell = Rc::clone(cell);
                                let setter_cell = Rc::clone(cell);
                                return Some(QValue::Left(Rc::new(RefCell::new(FieldValue::new(
                                    Box::new(move || getter_cell.borrow().clone()),
                                    Box::new(move |value| {
                                        *setter_cell.borrow_mut() = value;
                                        true
                                    }),
                                    None,
                                )))));
                            }
                            if let Some(value) = native_type.static_fields.get(&registered_name) {
                                return Some(QValue::Data(value.clone()));
                            }
                        }
                        None
                    }
                    // Java:bean 字段/getter 反射读取 → NativeObject 显式读取。
                    None => {
                        let type_name = obj.borrow().native_type_name().to_string();
                        let registered_name = self
                            .types
                            .get(&type_name)
                            .and_then(|native_type| {
                                PreferredFieldHandler::gather_field_recursive(
                                    native_type,
                                    field_name,
                                )
                            })
                            .unwrap_or_else(|| field_name.to_string());
                        if !skip_security && !self.check_member(&type_name, &registered_name) {
                            return None;
                        }
                        if let Some(native_type) = self.types.get(&type_name) {
                            if let (Some(getter), Some(setter)) = (
                                native_type.fields.get(&registered_name),
                                native_type.field_setters.get(&registered_name),
                            ) {
                                let getter = Rc::clone(getter);
                                let setter = Rc::clone(setter);
                                let getter_bean = bean.clone();
                                let setter_bean = bean.clone();
                                return Some(QValue::Left(Rc::new(RefCell::new(FieldValue::new(
                                    Box::new(move || {
                                        getter(&getter_bean).unwrap_or(DataValue::Null)
                                    }),
                                    Box::new(move |value| setter(&setter_bean, &value)),
                                    None,
                                )))));
                            }
                            if let Some(value) = native_type
                                .fields
                                .get(&registered_name)
                                .and_then(|getter| getter(bean))
                            {
                                return Some(QValue::Data(value));
                            }
                            // Rust 的显式注册表就是 Java 反射可见性边界：类型已
                            // 注册但成员未注册时，不得绕过注册表直读对象字段。
                            return None;
                        }
                        obj.borrow().get_field(&registered_name).map(QValue::Data)
                    }
                }
            }
            _ => {
                // 注册的实例字段(按 Java 类型名)。
                // 安全策略接线点:实例字段访问前过 QLSecurityStrategy。
                let type_name = bean.data_type_name();
                let native_type = self.types.get(type_name)?;
                let registered_name =
                    PreferredFieldHandler::gather_field_recursive(native_type, field_name)?;
                if !skip_security && !self.check_member(type_name, &registered_name) {
                    return None;
                }
                if let (Some(getter), Some(setter)) = (
                    native_type.fields.get(&registered_name),
                    native_type.field_setters.get(&registered_name),
                ) {
                    let getter = Rc::clone(getter);
                    let setter = Rc::clone(setter);
                    let getter_bean = bean.clone();
                    let setter_bean = bean.clone();
                    return Some(QValue::Left(Rc::new(RefCell::new(FieldValue::new(
                        Box::new(move || getter(&getter_bean).unwrap_or(DataValue::Null)),
                        Box::new(move |value| setter(&setter_bean, &value)),
                        None,
                    )))));
                }
                native_type
                    .fields
                    .get(&registered_name)
                    .and_then(|getter| getter(bean))
                    .map(QValue::Data)
            }
        }
    }

    // ---- 对应 Java ReflectLoader.loadMethod + member/MethodHandler ----

    /// 按名解析 `bean` 上的可调用方法。对应 Java `loadMethod` 返回 `null`
    /// 的语义:不存在时返回 `None`。
    ///
    /// # Arguments
    ///
    /// * `bean` - 实例接收者或表示静态类型的 MetaClass。
    /// * `method_name` - 待解析的方法名或 QL 别名。
    ///
    /// # Returns
    ///
    /// 扩展方法优先，其次为安全策略允许的内建或注册方法；均未命中时返回
    /// `None`。需要精确重载选择时应使用 [`NativeRegistry::resolve_method_for_args`]。
    pub fn resolve_method(&self, bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
        // MetaClass 接收者 → 静态方法(Java `isStaticMethod` 分支)。
        if let Some(meta) = as_meta_class(bean) {
            if method_name == "getName" {
                let class_name = meta.java_name().to_string();
                return Some(Rc::new(move |_bean, args| {
                    if args.is_empty() {
                        Ok(DataValue::Str(class_name.clone()))
                    } else {
                        Err(wrong_args("Class.getName"))
                    }
                }));
            }
            let name = meta.java_name();
            let native_type = self.types.get(name)?;
            let registered_name =
                Self::resolve_registered_method_name(native_type, method_name, true);
            // 安全策略接线点:静态方法访问前过 QLSecurityStrategy。
            if !self.check_member(name, registered_name) {
                return None;
            }
            return native_type
                .static_methods
                .get(registered_name)
                .map(Rc::clone);
        }
        let type_name = native_type_name(bean);
        // Java 先解析扩展函数，再判断是否为隔离策略。
        if let Some(method) = self
            .resolve_extension_method(bean, method_name)
            .or_else(|| builtin_extension_method(bean, method_name))
        {
            return Some(method);
        }
        let native_type = self.types.get(&type_name);
        let registered_name = native_type
            .map(|native_type| {
                Self::resolve_registered_method_name(native_type, method_name, false)
            })
            .unwrap_or(method_name);
        // Java 反射方法（含 Rust 内建 JDK 方法子集）统一通过安全策略；
        // 别名先还原为真实成员名，再执行与 Java 反射 Member 相同的检查。
        if !self.check_member(&type_name, registered_name) {
            return None;
        }
        if let Some(method) = builtin_method(bean, method_name) {
            return Some(method);
        }
        if let Some(method) = native_type
            .and_then(|native_type| native_type.methods.get(registered_name).map(Rc::clone))
        {
            return Some(method);
        }
        None
    }

    /// 按调用现场实参选择同名方法候选。对应 Java
    /// `ReflectLoader#loadMethod(bean, name, argTypes, ...)`。
    ///
    /// # Arguments
    ///
    /// * `bean` - 实例接收者或静态类型对象。
    /// * `method_name` - 脚本调用的方法名或别名。
    /// * `args` - 用于重载选择和类型转换的运行时实参。
    ///
    /// # Returns
    ///
    /// 返回安全策略允许的最佳方法闭包；无匹配项时返回 `None`。
    pub fn resolve_method_for_args(
        &self,
        bean: &DataValue,
        method_name: &str,
        args: &[DataValue],
    ) -> Option<NativeMethod> {
        if let Some(meta) = as_meta_class(bean) {
            if method_name == "getName" && args.is_empty() {
                let class_name = meta.java_name().to_string();
                return Some(Rc::new(move |_bean, _args| {
                    Ok(DataValue::Str(class_name.clone()))
                }));
            }
            return self.resolve_registered_candidate(
                meta.java_name(),
                method_name,
                args,
                true,
                bean,
            );
        }

        let type_name = native_type_name(bean);
        // Java 扩展函数优先于反射成员。
        if let Some(method) = self
            .resolve_extension_method(bean, method_name)
            .or_else(|| builtin_extension_method(bean, method_name))
        {
            return Some(method);
        }

        if let Some(method) =
            self.resolve_registered_candidate(&type_name, method_name, args, false, bean)
        {
            return Some(method);
        }
        // 未显式登记候选时继续兼容内建方法表。
        if self.types.get(&type_name).is_none_or(|native_type| {
            native_type
                .method_candidates
                .get(method_name)
                .is_none_or(Vec::is_empty)
        }) && self.check_member(&type_name, method_name)
        {
            return builtin_method(bean, method_name);
        }
        None
    }

    /// 按 Java `declaringClass.isAssignableFrom(bean.getClass())` 解析扩展
    /// 函数；不能只用运行时类型名做精确 HashMap 命中，否则注册在
    /// `Number` / `List` 上的扩展无法用于 `Integer` / `ArrayList`。
    fn resolve_extension_method(
        &self,
        bean: &DataValue,
        method_name: &str,
    ) -> Option<NativeMethod> {
        let type_name = native_type_name(bean);
        if let Some(method) = self
            .extension_methods
            .get(&(type_name.clone(), method_name.to_string()))
        {
            return Some(Rc::clone(method));
        }
        let argument_type = runtime_class_ref(bean);
        self.extension_methods
            .iter()
            .find_map(|((declaring_type, registered_name), method)| {
                (registered_name == method_name
                    && self.is_assignable(&ClassRef::Named(declaring_type.clone()), &argument_type))
                .then(|| Rc::clone(method))
            })
    }

    fn resolve_registered_candidate(
        &self,
        type_name: &str,
        method_name: &str,
        args: &[DataValue],
        is_static: bool,
        bean: &DataValue,
    ) -> Option<NativeMethod> {
        let native_type = self.types.get(type_name)?;
        let registered_name =
            Self::resolve_registered_method_name(native_type, method_name, is_static);
        if !self.check_member(type_name, registered_name) {
            return None;
        }
        let candidates = if is_static {
            native_type.static_method_candidates.get(registered_name)
        } else {
            native_type.method_candidates.get(registered_name)
        };
        if let Some(candidates) = candidates {
            if let Some(candidate) = self.select_method_candidate(candidates, args) {
                return Some(wrap_method_candidate(candidate));
            }
        }

        let legacy = if is_static {
            native_type.static_methods.get(registered_name)
        } else {
            native_type.methods.get(registered_name)
        };
        if let Some(method) = legacy {
            return Some(Rc::clone(method));
        }

        // Java 从实际声明类开始逐层查找；当前类有同名候选但不匹配时，
        // 继续父类，保留 override/hiding 与 fallback 的组合语义。
        for supertype in &native_type.supertypes {
            if let Some(method) =
                self.resolve_registered_candidate(supertype, method_name, args, is_static, bean)
            {
                return Some(method);
            }
        }
        let _ = bean;
        None
    }

    fn select_method_candidate<'a>(
        &self,
        candidates: &'a [NativeMethodCandidate],
        args: &[DataValue],
    ) -> Option<&'a NativeMethodCandidate> {
        let signatures: Vec<(Vec<ClassRef>, bool)> = candidates
            .iter()
            .map(|candidate| (candidate.parameter_types.clone(), candidate.var_args))
            .collect();
        let arg_types = crate::utils::basic_util::BasicUtil::get_type_of_object(args);
        let index = MemberResolver::resolve_candidate_index_with_function_interface(
            &signatures,
            &arg_types,
            |param, arg| self.is_assignable(param, arg),
            |param| self.is_function_interface(param),
        )?;
        candidates.get(index)
    }

    fn select_constructor_candidate<'a>(
        &self,
        candidates: &'a [NativeConstructorCandidate],
        args: &[DataValue],
    ) -> Option<&'a NativeConstructorCandidate> {
        let signatures: Vec<(Vec<ClassRef>, bool)> = candidates
            .iter()
            .map(|candidate| (candidate.parameter_types.clone(), candidate.var_args))
            .collect();
        let arg_types = crate::utils::basic_util::BasicUtil::get_type_of_object(args);
        let index = MemberResolver::resolve_constructor(
            &signatures,
            &arg_types,
            |param, arg| self.is_assignable(param, arg),
            |param| self.is_function_interface(param),
        )?;
        candidates.get(index)
    }

    /// 判断形参类型是否为函数式接口。
    ///
    /// JDK 内建函数接口按规范名识别；宿主自定义接口由 [`NativeType`] 的
    /// `is_interface + abstract_methods` 元数据判定并通过 [`CacheUtil`]
    /// 缓存。对应 Java `CacheUtil.isFunctionInterface(Class<?>)`。
    fn is_function_interface(&self, class_ref: &ClassRef) -> bool {
        let name = class_ref.java_name();
        if name.starts_with("java.util.function.") || name == "java.lang.Runnable" {
            return true;
        }
        self.types
            .get(name)
            .is_some_and(|native_type| {
                self.function_interface_cache
                    .is_function_interface(native_type)
            })
    }

    fn is_assignable(&self, param: &ClassRef, arg: &ClassRef) -> bool {
        if param == arg || param.is_java_object() {
            return true;
        }
        let param_name = param.java_name();
        let arg_name = arg.java_name();
        if param_name == "java.lang.Number" && is_numeric_java_name(arg_name) {
            return true;
        }
        if let (Some(param_item), Some(arg_item)) =
            (param_name.strip_suffix("[]"), arg_name.strip_suffix("[]"))
        {
            return self.is_assignable(
                &ClassRef::Named(param_item.to_string()),
                &ClassRef::Named(arg_item.to_string()),
            );
        }
        self.type_extends(arg_name, param_name, &mut Vec::new())
    }

    fn type_extends(
        &self,
        type_name: &str,
        expected_supertype: &str,
        visited: &mut Vec<String>,
    ) -> bool {
        if type_name == expected_supertype {
            return true;
        }
        if visited.iter().any(|visited_name| visited_name == type_name) {
            return false;
        }
        visited.push(type_name.to_string());
        self.types.get(type_name).is_some_and(|native_type| {
            native_type.supertypes.iter().any(|supertype| {
                supertype == expected_supertype
                    || self.type_extends(supertype, expected_supertype, visited)
            })
        })
    }

    /// 注册内建锚点类型,供 `ClassSupplier` 式查询与宿主覆盖挂接;
    /// 实际分派在 [`builtin_method`]。对应 Java 中这些 JDK 类天然可被反射。
    fn register_builtin_types(&mut self) {
        let mut system = NativeType::named("java.lang.System");
        system.static_methods.insert(
            "currentTimeMillis".to_string(),
            Rc::new(|_bean, args| {
                if args.is_empty() {
                    Ok(DataValue::Long(current_time_millis()))
                } else {
                    Err(wrong_args("System.currentTimeMillis"))
                }
            }),
        );
        self.register_type(system);

        let mut array_list = NativeType::named("java.util.ArrayList");
        array_list.constructor = Some(Rc::new(|args| match args {
            [] => Ok(DataValue::list(Vec::new())),
            [capacity] if capacity.is_number() => {
                let capacity = crate::runtime::data::convert::to_i64(capacity);
                if capacity < 0 {
                    Err(wrong_args("ArrayList"))
                } else {
                    Ok(DataValue::List(Rc::new(RefCell::new(Vec::with_capacity(
                        capacity as usize,
                    )))))
                }
            }
            _ => Err(wrong_args("ArrayList")),
        }));
        self.register_type(array_list);

        let map_constructor = Rc::new(|args: &[DataValue]| {
            if args.is_empty() {
                Ok(DataValue::Map(Rc::new(RefCell::new(IndexMap::new()))))
            } else {
                Err(wrong_args("HashMap"))
            }
        });
        let mut hash_map = NativeType::named("java.util.HashMap");
        hash_map.supertypes = vec!["java.util.Map".to_string(), "java.lang.Object".to_string()];
        hash_map.constructor = Some(map_constructor.clone());
        self.register_type(hash_map);

        let mut linked_hash_map = NativeType::named("java.util.LinkedHashMap");
        linked_hash_map.supertypes = vec!["java.util.HashMap".to_string()];
        linked_hash_map.constructor = Some(map_constructor);
        self.register_type(linked_hash_map);

        let mut collectors = NativeType::named("java.util.stream.Collectors");
        collectors.static_methods.insert(
            "toList".to_string(),
            Rc::new(|_bean, args| {
                if args.is_empty() {
                    Ok(JavaCollector.into_data_value())
                } else {
                    Err(wrong_args("Collectors.toList"))
                }
            }),
        );
        self.register_type(collectors);

        // Java 流的实际实现类通常不是公开的 Stream 接口本身；
        // Java MemberResolver 会沿接口查找 filter/map/collect。Rust 用
        // Stream 锚点类型登记同名候选，让真实调用现场经过
        // MemberResolver 的 Lambda/精确类型匹配后再进入宿主对象。
        let mut stream = NativeType::named("java.util.stream.Stream");
        stream.add_method_candidate(
            "filter",
            NativeMethodCandidate::new(
                vec![ClassRef::Named("java.util.function.Predicate".to_string())],
                false,
                native_object_method("filter"),
            ),
        );
        stream.add_method_candidate(
            "map",
            NativeMethodCandidate::new(
                vec![ClassRef::Named("java.util.function.Function".to_string())],
                false,
                native_object_method("map"),
            ),
        );
        stream.add_method_candidate(
            "collect",
            NativeMethodCandidate::new(
                vec![ClassRef::Named("java.util.stream.Collector".to_string())],
                false,
                native_object_method("collect"),
            ),
        );
        self.register_type(stream);

        let mut hash_set = NativeType::named("java.util.HashSet");
        hash_set.constructor = Some(Rc::new(|args| {
            if args.is_empty() {
                Ok(OpaqueNativeObject::new("java.util.HashSet").into_data_value())
            } else {
                Err(wrong_args("HashSet"))
            }
        }));
        self.register_type(hash_set);

        let mut integer = NativeType::named("java.lang.Integer");
        integer
            .static_fields
            .insert("MAX_VALUE".to_string(), DataValue::Int(i32::MAX));
        integer
            .static_fields
            .insert("MIN_VALUE".to_string(), DataValue::Int(i32::MIN));
        integer.constructor = Some(Rc::new(|args| match args {
            [value] if value.is_number() => Ok(DataValue::Int(
                crate::runtime::data::convert::to_i64(value) as i32,
            )),
            _ => Err(wrong_args("Integer")),
        }));
        self.register_type(integer);

        let mut long = NativeType::named("java.lang.Long");
        long.static_fields
            .insert("MAX_VALUE".to_string(), DataValue::Long(i64::MAX));
        long.static_fields
            .insert("MIN_VALUE".to_string(), DataValue::Long(i64::MIN));
        long.constructor = Some(Rc::new(|args| match args {
            [value] if value.is_number() => Ok(DataValue::Long(
                crate::runtime::data::convert::to_i64(value),
            )),
            _ => Err(wrong_args("Long")),
        }));
        self.register_type(long);

        let mut big_integer = NativeType::named("java.math.BigInteger");
        big_integer.static_methods.insert(
            "valueOf".to_string(),
            Rc::new(|_bean, args| match args {
                [value] if value.is_number() => Ok(DataValue::BigInt(
                    crate::runtime::data::convert::to_big_int(value),
                )),
                _ => Err(wrong_args("BigInteger.valueOf")),
            }),
        );
        self.register_type(big_integer);

        for exception_name in [
            "java.lang.RuntimeException",
            "java.lang.NullPointerException",
        ] {
            let mut exception_type = NativeType::named(exception_name);
            exception_type.constructor = Some(Rc::new(move |args| {
                if args.is_empty() || matches!(args, [DataValue::Str(_)]) {
                    Ok(OpaqueNativeObject::new(exception_name).into_data_value())
                } else {
                    Err(wrong_args(exception_name))
                }
            }));
            self.register_type(exception_type);
        }

        for name in ["java.lang.String", "java.lang.Double", "java.lang.Boolean"] {
            self.register_type(NativeType::named(name));
        }
    }
}

/// 将显式登记的 Java 接口候选转发到对应宿主对象。
///
/// 候选选择已由 [`MemberResolver`] 完成；此处只承担 Java
/// `Method.invoke` 的调用阶段。
fn native_object_method(method_name: &'static str) -> NativeMethod {
    Rc::new(move |bean, args| match bean {
        DataValue::Object(object) => object.borrow_mut().call_method(method_name, args),
        _ => Err(wrong_args(method_name)),
    })
}

/// 返回值的 Java 运行时类型名；宿主对象使用其显式注册的原生类名。
///
/// 对应 Java `bean.getClass().getName()`。
fn native_type_name(bean: &DataValue) -> String {
    match bean {
        DataValue::Object(object) => object.borrow().native_type_name().to_string(),
        _ => bean.data_type_name().to_string(),
    }
}

fn runtime_class_ref(value: &DataValue) -> ClassRef {
    crate::utils::basic_util::BasicUtil::type_of_value(value)
}

fn is_numeric_java_name(name: &str) -> bool {
    matches!(
        name,
        "java.lang.Byte"
            | "java.lang.Short"
            | "java.lang.Integer"
            | "java.lang.Long"
            | "java.lang.Float"
            | "java.lang.Double"
            | "java.math.BigInteger"
            | "java.math.BigDecimal"
    )
}

fn convert_candidate_arguments(
    arguments: &[DataValue],
    parameter_types: &[ClassRef],
    var_args: bool,
) -> Vec<DataValue> {
    let target_types = parameter_types
        .iter()
        .map(ClassRef::to_target_type)
        .collect::<Vec<_>>();
    ParametersTypeConvertor::cast(arguments, &target_types, var_args)
}

fn wrap_method_candidate(candidate: &NativeMethodCandidate) -> NativeMethod {
    let method = Rc::clone(&candidate.method);
    let parameter_types = candidate.parameter_types.clone();
    let var_args = candidate.var_args;
    Rc::new(move |bean, arguments| {
        let converted = convert_candidate_arguments(arguments, &parameter_types, var_args);
        method(bean, &converted)
    })
}

// ---------------------------------------------------------------------------
// 内建方法子集(SPEC §4:String/List/Map/数组/数值/布尔 常用方法,
// 对齐 Java 版脚本可直接调用的 JDK 方法)。
// ---------------------------------------------------------------------------

/// 取第 `i` 个整数参数(Java 侧由反射按 `int` 参数类型自动拆箱转换)。
fn int_arg(args: &[DataValue], i: usize) -> Option<i64> {
    args.get(i).and_then(|v| {
        if v.is_number() {
            Some(crate::runtime::data::convert::to_i64(v))
        } else {
            None
        }
    })
}

/// 取第 `i` 个字符串参数(Java `String.valueOf(arg)`)。
fn str_arg(args: &[DataValue], i: usize) -> Option<String> {
    args.get(i).map(|v| v.string_value_of())
}

/// 参数不匹配错误,对应 Java `QLErrorCodes.INVOKE_METHOD_WITH_WRONG_ARGUMENTS`。
fn wrong_args(method: &str) -> QLException {
    QLException::for_test(
        QLExceptionKind::Runtime,
        format!("invoke method '{method}' with wrong arguments"),
        error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
    )
}

/// 内建方法分派:对应 Java 中 String/List/Map/Number/Boolean 上真实存在、
/// 脚本可直接调用的方法集合。
fn builtin_method(bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
    match bean {
        DataValue::Str(_) => string_method(method_name),
        DataValue::List(_) => list_method(method_name),
        DataValue::Map(_) => map_method(method_name),
        v if v.is_number() => number_method(method_name),
        DataValue::Bool(_) => bool_method(method_name),
        _ => None,
    }
}

/// Java 默认扩展函数分派。`ReflectLoader` 在隔离策略判断之前解析
/// `FilterExtensionFunction` / `MapExtensionFunction`。
fn builtin_extension_method(bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
    if !matches!(bean, DataValue::List(_)) {
        return None;
    }
    match method_name {
        "filter" => Some(Rc::new(|bean, args| {
            crate::runtime::function::FilterExtensionFunction::instance().invoke(bean, args)
        })),
        "map" => Some(Rc::new(|bean, args| {
            crate::runtime::function::MapExtensionFunction::instance().invoke(bean, args)
        })),
        _ => None,
    }
}

/// `java.lang.String` 的脚本可用方法子集。
fn string_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "length" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Int(s.chars().count() as i32)),
            _ => Err(wrong_args("length")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Bool(s.is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "charAt" => Rc::new(|bean, args| match (bean, int_arg(args, 0)) {
            (DataValue::Str(s), Some(i)) => s
                .chars()
                .nth(i as usize)
                .map(DataValue::Char)
                .ok_or_else(|| wrong_args("charAt")),
            _ => Err(wrong_args("charAt")),
        }),
        "contains" => Rc::new(|bean, args| match (bean, str_arg(args, 0)) {
            (DataValue::Str(s), Some(sub)) => Ok(DataValue::Bool(s.contains(&sub))),
            _ => Err(wrong_args("contains")),
        }),
        "startsWith" => Rc::new(|bean, args| match (bean, str_arg(args, 0)) {
            (DataValue::Str(s), Some(p)) => Ok(DataValue::Bool(s.starts_with(&p))),
            _ => Err(wrong_args("startsWith")),
        }),
        "endsWith" => Rc::new(|bean, args| match (bean, str_arg(args, 0)) {
            (DataValue::Str(s), Some(p)) => Ok(DataValue::Bool(s.ends_with(&p))),
            _ => Err(wrong_args("endsWith")),
        }),
        "indexOf" => Rc::new(|bean, args| match (bean, str_arg(args, 0)) {
            (DataValue::Str(s), Some(sub)) => {
                Ok(DataValue::Int(s.find(&sub).map(|i| i as i32).unwrap_or(-1)))
            }
            _ => Err(wrong_args("indexOf")),
        }),
        "toUpperCase" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Str(s.to_uppercase())),
            _ => Err(wrong_args("toUpperCase")),
        }),
        "toLowerCase" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Str(s.to_lowercase())),
            _ => Err(wrong_args("toLowerCase")),
        }),
        "trim" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Str(s.trim().to_string())),
            _ => Err(wrong_args("trim")),
        }),
        "substring" => Rc::new(|bean, args| match bean {
            DataValue::Str(s) => {
                let chars: Vec<char> = s.chars().collect();
                let begin = int_arg(args, 0).unwrap_or(0).max(0) as usize;
                let end = int_arg(args, 1)
                    .map(|e| e.max(0) as usize)
                    .unwrap_or(chars.len());
                if begin > chars.len() || end > chars.len() || begin > end {
                    return Err(wrong_args("substring"));
                }
                Ok(DataValue::Str(chars[begin..end].iter().collect()))
            }
            _ => Err(wrong_args("substring")),
        }),
        "replace" => Rc::new(
            |bean, args| match (bean, str_arg(args, 0), str_arg(args, 1)) {
                (DataValue::Str(s), Some(from), Some(to)) => {
                    Ok(DataValue::Str(s.replace(&from, &to)))
                }
                _ => Err(wrong_args("replace")),
            },
        ),
        "equals" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Str(s), Some(DataValue::Str(o))) => Ok(DataValue::Bool(s == o)),
            (DataValue::Str(_), Some(_)) => Ok(DataValue::Bool(false)),
            _ => Err(wrong_args("equals")),
        }),
        "equalsIgnoreCase" => Rc::new(|bean, args| match (bean, str_arg(args, 0)) {
            (DataValue::Str(s), Some(o)) => Ok(DataValue::Bool(s.eq_ignore_ascii_case(&o))),
            _ => Err(wrong_args("equalsIgnoreCase")),
        }),
        "split" => Rc::new(|bean, args| match (bean, str_arg(args, 0)) {
            (DataValue::Str(s), Some(sep)) => Ok(DataValue::list(
                s.split(&sep)
                    .map(|p| DataValue::Str(p.to_string()))
                    .collect(),
            )),
            _ => Err(wrong_args("split")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.string_value_of()))),
        _ => return None,
    };
    Some(f)
}

/// `java.util.List` 的脚本可用方法子集。
fn list_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "size" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => Ok(DataValue::Int(l.borrow().len() as i32)),
            _ => Err(wrong_args("size")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => Ok(DataValue::Bool(l.borrow().is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "parallelStream" | "stream" => Rc::new(|bean, args| match bean {
            DataValue::List(items) if args.is_empty() => {
                Ok(JavaStream::new(items.borrow().clone()).into_data_value())
            }
            _ => Err(wrong_args("parallelStream")),
        }),
        "get" => Rc::new(|bean, args| match (bean, int_arg(args, 0)) {
            (DataValue::List(l), Some(i)) => {
                let list = l.borrow();
                let idx = if i < 0 { i + list.len() as i64 } else { i };
                list.get(idx as usize).cloned().ok_or_else(|| {
                    QLException::host_error(
                        QLExceptionKind::Runtime,
                        format!("Index {i} out of bounds for length {}", list.len()),
                        error_codes::INDEX_OUT_BOUND,
                    )
                })
            }
            _ => Err(wrong_args("get")),
        }),
        "add" => Rc::new(|bean, args| match bean {
            DataValue::List(l) => {
                l.borrow_mut()
                    .push(args.first().cloned().unwrap_or(DataValue::Null));
                Ok(DataValue::Bool(true))
            }
            _ => Err(wrong_args("add")),
        }),
        "set" => Rc::new(|bean, args| match (bean, int_arg(args, 0), args.get(1)) {
            (DataValue::List(l), Some(i), Some(v)) => {
                let mut list = l.borrow_mut();
                let idx = if i < 0 { i + list.len() as i64 } else { i } as usize;
                if idx >= list.len() {
                    return Err(wrong_args("set"));
                }
                Ok(std::mem::replace(&mut list[idx], v.clone()))
            }
            _ => Err(wrong_args("set")),
        }),
        "remove" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(index)) if index.is_number() => {
                let mut list = l.borrow_mut();
                let i = crate::runtime::data::convert::to_i64(index);
                let idx = if i < 0 { i + list.len() as i64 } else { i } as usize;
                if idx >= list.len() {
                    return Err(wrong_args("remove"));
                }
                Ok(list.remove(idx))
            }
            (DataValue::List(l), Some(target)) => {
                let mut list = l.borrow_mut();
                match list.iter().position(|v| v == target) {
                    Some(pos) => {
                        list.remove(pos);
                        Ok(DataValue::Bool(true))
                    }
                    None => Ok(DataValue::Bool(false)),
                }
            }
            _ => Err(wrong_args("remove")),
        }),
        "contains" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(target)) => {
                Ok(DataValue::Bool(l.borrow().iter().any(|v| v == target)))
            }
            _ => Err(wrong_args("contains")),
        }),
        "indexOf" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(target)) => Ok(DataValue::Int(
                l.borrow()
                    .iter()
                    .position(|v| v == target)
                    .map(|p| p as i32)
                    .unwrap_or(-1),
            )),
            _ => Err(wrong_args("indexOf")),
        }),
        "addAll" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(DataValue::List(other))) => {
                let items = other.borrow().clone();
                l.borrow_mut().extend(items);
                Ok(DataValue::Bool(true))
            }
            _ => Err(wrong_args("addAll")),
        }),
        "clear" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => {
                l.borrow_mut().clear();
                Ok(DataValue::Null)
            }
            _ => Err(wrong_args("clear")),
        }),
        "subList" => Rc::new(
            |bean, args| match (bean, int_arg(args, 0), int_arg(args, 1)) {
                (DataValue::List(l), Some(a), Some(b)) => {
                    let list = l.borrow();
                    let (a, b) = (a.max(0) as usize, (b.max(0) as usize).min(list.len()));
                    if a > b {
                        return Err(wrong_args("subList"));
                    }
                    Ok(DataValue::list(list[a..b].to_vec()))
                }
                _ => Err(wrong_args("subList")),
            },
        ),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.string_value_of()))),
        _ => return None,
    };
    Some(f)
}

/// `java.util.Map` 的脚本可用方法子集。
fn map_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "entrySet" => Rc::new(|bean, args| match bean {
            DataValue::Map(map) if args.is_empty() => Ok(DataValue::list(
                map.borrow()
                    .entries()
                    .iter()
                    .map(|(key, value)| {
                        JavaMapEntry::new(key.clone(), value.clone()).into_data_value()
                    })
                    .collect(),
            )),
            _ => Err(wrong_args("entrySet")),
        }),
        "size" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::Int(m.borrow().len() as i32)),
            _ => Err(wrong_args("size")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::Bool(m.borrow().is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "get" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(k)) => {
                Ok(m.borrow().get(k).cloned().unwrap_or(DataValue::Null))
            }
            _ => Err(wrong_args("get")),
        }),
        "put" => Rc::new(|bean, args| match (bean, args.first(), args.get(1)) {
            (DataValue::Map(m), Some(k), Some(v)) => Ok(m
                .borrow_mut()
                .insert(k.clone(), v.clone())
                .unwrap_or(DataValue::Null)),
            _ => Err(wrong_args("put")),
        }),
        "containsKey" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(k)) => Ok(DataValue::Bool(m.borrow().contains_key(k))),
            _ => Err(wrong_args("containsKey")),
        }),
        "containsValue" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(v)) => {
                Ok(DataValue::Bool(m.borrow().values().any(|value| value == v)))
            }
            _ => Err(wrong_args("containsValue")),
        }),
        "remove" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(k)) => Ok(m.borrow_mut().remove(k).unwrap_or(DataValue::Null)),
            _ => Err(wrong_args("remove")),
        }),
        "keySet" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::list(m.borrow().keys().cloned().collect())),
            _ => Err(wrong_args("keySet")),
        }),
        "values" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::list(m.borrow().values().cloned().collect())),
            _ => Err(wrong_args("values")),
        }),
        "clear" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => {
                m.borrow_mut().clear();
                Ok(DataValue::Null)
            }
            _ => Err(wrong_args("clear")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.string_value_of()))),
        _ => return None,
    };
    Some(f)
}

/// `java.lang.Number` 子类的脚本可用方法子集。
fn number_method(name: &str) -> Option<NativeMethod> {
    use crate::runtime::data::convert::{to_f64, to_i64};
    let f: NativeMethod = match name {
        "intValue" => Rc::new(|bean, _| Ok(DataValue::Int(to_i64(bean) as i32))),
        "longValue" => Rc::new(|bean, _| Ok(DataValue::Long(to_i64(bean)))),
        "doubleValue" => Rc::new(|bean, _| Ok(DataValue::Double(to_f64(bean)))),
        "floatValue" => Rc::new(|bean, _| Ok(DataValue::Float(to_f64(bean) as f32))),
        "shortValue" => Rc::new(|bean, _| Ok(DataValue::Short(to_i64(bean) as i16))),
        "byteValue" => Rc::new(|bean, _| Ok(DataValue::Byte(to_i64(bean) as i8))),
        "compareTo" => Rc::new(|bean, args| match args.first() {
            Some(other) if other.is_number() => {
                let ord = number_compare(bean, other).unwrap_or(std::cmp::Ordering::Equal);
                Ok(DataValue::Int(match ord {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }))
            }
            _ => Err(wrong_args("compareTo")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.string_value_of()))),
        _ => return None,
    };
    Some(f)
}

/// `java.lang.Boolean` 的脚本可用方法子集。
fn bool_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "booleanValue" => Rc::new(|bean, _| match bean {
            DataValue::Bool(b) => Ok(DataValue::Bool(*b)),
            _ => Err(wrong_args("booleanValue")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.string_value_of()))),
        _ => return None,
    };
    Some(f)
}
