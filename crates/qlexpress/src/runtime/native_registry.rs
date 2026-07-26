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
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::convert::number_compare;
use crate::runtime::data::MapItemValue;
use crate::runtime::function::ExtensionFunction;
use crate::runtime::meta_class::{as_meta_class, MetaClass};
use crate::runtime::native_type::{NativeConstructor, NativeMethod, NativeType};
use crate::runtime::value::{DataValue, QValue};
use crate::security::ql_security_strategy::{NativeMember, QLSecurityStrategy};
use crate::utils::basic_util;

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
pub struct NativeRegistry {
    /// 类型名 -> 注册类型(Java 侧为 `ClassLoader` 可加载的所有类)。
    types: HashMap<String, NativeType>,
    /// 成员访问安全策略(Java `ReflectLoader.securityStrategy`)。
    /// `RefCell`:注册表经 `Rc` 共享给 QVM,策略需在 runner 层可改。
    security_strategy: RefCell<QLSecurityStrategy>,
}

impl NativeRegistry {
    /// 空注册表。对应 Java `new ReflectLoader()`(无任何已知类型)。
    pub fn new() -> Self {
        NativeRegistry {
            types: HashMap::new(),
            // 注册表裸用时默认放行;Express4Runner 构造时按
            // `InitOptions.securityStrategy` 覆盖(Java 默认 `isolation`)。
            security_strategy: RefCell::new(QLSecurityStrategy::open()),
        }
    }

    /// 设置成员访问安全策略。对应 Java `ReflectLoader` 持有的
    /// `securityStrategy`(由 `InitOptions` 注入)。
    pub fn set_security_strategy(&self, security_strategy: QLSecurityStrategy) {
        *self.security_strategy.borrow_mut() = security_strategy;
    }

    /// 当前安全策略。对应 Java `InitOptions.getSecurityStrategy()` 经
    /// `ReflectLoader` 读取的值。
    pub fn security_strategy(&self) -> QLSecurityStrategy {
        self.security_strategy.borrow().clone()
    }

    /// Java `ReflectLoader.check(Member)`:`check` 不通过即视为成员不存在。
    fn check_member(&self, type_name: &str, member_name: &str) -> bool {
        self.security_strategy
            .borrow()
            .check(&NativeMember::new(type_name, member_name))
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
    pub fn with_builtins() -> Self {
        let mut registry = NativeRegistry::new();
        registry.register_builtin_types();
        registry
    }

    /// 注册类型。对应 Java `ClassSupplier.addClass` 一类的类型供给。
    pub fn register_type(&mut self, native_type: NativeType) {
        self.types.insert(native_type.name.clone(), native_type);
    }

    /// 按名取注册类型。对应 Java `Class.forName` 命中已供给类型。
    pub fn get_type(&self, name: &str) -> Option<&NativeType> {
        self.types.get(name)
    }

    /// 为指定类型追加(或覆盖)一个实例方法,对应 Java
    /// `ReflectLoader.addExtendFunction` 的注册效果(扩展函数进入成员
    /// 分派路径,脚本以 `target.method(...)` 调用)。
    /// 类型未注册时先创建空注册项(Java 中任何可加载类天然存在)。
    pub fn register_method(
        &mut self,
        type_name: impl Into<String>,
        method_name: impl Into<String>,
        method: NativeMethod,
    ) {
        let type_name = type_name.into();
        self.types
            .entry(type_name.clone())
            .or_insert_with(|| NativeType::named(type_name))
            .methods
            .insert(method_name.into(), method);
    }

    // ---- 对应 Java ReflectLoader.loadConstructor ----

    /// 对应 Java 方法 `loadConstructor(Class, Class[])`:取注册构造器;
    /// 参数匹配委托给构造器闭包自身(Java 由 `MemberResolver` 选重载,
    /// Rust 一个类型只注册一个构造入口)。
    pub fn load_constructor(&self, clz: &ClassRef) -> Option<NativeConstructor> {
        match clz {
            ClassRef::Named(name) => self
                .types
                .get(name)
                .and_then(|t| t.constructor.as_ref().map(Rc::clone)),
            ClassRef::Primitive(_) => None,
        }
    }

    // ---- 对应 Java ReflectLoader.loadField ----

    /// 对应 Java 方法 `loadField(Object bean, String fieldName, boolean
    /// skipSecurity, ErrorReporter)`:字段不存在时返回 `None`(Java 返回 `null`)。
    pub fn load_field(&self, bean: &DataValue, field_name: &str) -> Option<QValue> {
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
            DataValue::Map(map) => Some(QValue::Left(Rc::new(RefCell::new(
                MapItemValue::new(Rc::clone(map), DataValue::Str(field_name.to_string())),
            )))),
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
                        if let ClassRef::Named(name) = clz {
                            // 安全策略接线点(Java ReflectLoader.check):
                            // 静态字段访问前过 QLSecurityStrategy。
                            if self.check_member(&name, field_name) {
                                if let Some(value) = self
                                    .types
                                    .get(&name)
                                    .and_then(|t| t.static_fields.get(field_name))
                                {
                                    return Some(QValue::Data(value.clone()));
                                }
                            }
                        }
                        None
                    }
                    // Java:bean 字段/getter 反射读取 → NativeObject 显式读取。
                    None => obj.borrow().get_field(field_name).map(QValue::Data),
                }
            }
            _ => {
                // 注册的实例字段(按 Java 类型名)。
                // 安全策略接线点:实例字段访问前过 QLSecurityStrategy。
                let type_name = bean.data_type_name();
                if !self.check_member(type_name, field_name) {
                    return None;
                }
                self.types
                    .get(type_name)
                    .and_then(|t| t.fields.get(field_name))
                    .and_then(|getter| getter(bean))
                    .map(QValue::Data)
            }
        }
    }

    // ---- 对应 Java ReflectLoader.loadMethod + member/MethodHandler ----

    /// 按名解析 `bean` 上的可调用方法。对应 Java `loadMethod` 返回 `null`
    /// 的语义:不存在时返回 `None`。
    pub fn resolve_method(&self, bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
        // MetaClass 接收者 → 静态方法(Java `isStaticMethod` 分支)。
        if let Some(meta) = as_meta_class(bean) {
            if let ClassRef::Named(name) = meta {
                // 安全策略接线点:静态方法访问前过 QLSecurityStrategy。
                if !self.check_member(&name, method_name) {
                    return None;
                }
                return self
                    .types
                    .get(&name)
                    .and_then(|t| t.static_methods.get(method_name).map(Rc::clone));
            }
            return None;
        }
        // 内建方法子集(Java 中即 String/List/Map/Number 的真实方法)。
        // 偏差:内建方法不过安全策略(Rust 语言内核;Java 默认 isolation
        // 下这些方法也会被拦,见类型文档)。
        if let Some(method) = builtin_method(bean, method_name) {
            return Some(method);
        }
        let type_name = bean.data_type_name();
        // 安全策略接线点:注册类型的实例方法访问前过 QLSecurityStrategy。
        if !self.check_member(type_name, method_name) {
            return None;
        }
        if let Some(method) = self
            .types
            .get(type_name)
            .and_then(|t| t.methods.get(method_name).map(Rc::clone))
        {
            return Some(method);
        }
        None
    }

    /// 注册内建锚点类型,供 `ClassSupplier` 式查询与宿主覆盖挂接;
    /// 实际分派在 [`builtin_method`]。对应 Java 中这些 JDK 类天然可被反射。
    fn register_builtin_types(&mut self) {
        for name in [
            "java.lang.String",
            "java.util.ArrayList",
            "java.util.LinkedHashMap",
            "java.lang.Integer",
            "java.lang.Long",
            "java.lang.Double",
            "java.lang.Boolean",
        ] {
            self.register_type(NativeType::named(name));
        }
    }
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
            (DataValue::Str(s), Some(sub)) => Ok(DataValue::Int(
                s.find(&sub).map(|i| i as i32).unwrap_or(-1),
            )),
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
        "replace" => Rc::new(|bean, args| match (bean, str_arg(args, 0), str_arg(args, 1)) {
            (DataValue::Str(s), Some(from), Some(to)) => {
                Ok(DataValue::Str(s.replace(&from, &to)))
            }
            _ => Err(wrong_args("replace")),
        }),
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
                s.split(&sep).map(|p| DataValue::Str(p.to_string())).collect(),
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
        // Java `ReflectLoader.defaultExtendFunctions` 默认注册
        // `FilterExtensionFunction.INSTANCE` / `MapExtensionFunction.INSTANCE`
        // (声明类 `java.util.List`);Rust 在此挂接同一语义。
        // (对齐测试 extensionfunction/extension_function.ql、
        // doc/list_map_filter.ql 发现遗漏。)
        "filter" => Rc::new(|bean, args| {
            crate::runtime::function::FilterExtensionFunction::instance().invoke(bean, args)
        }),
        "map" => Rc::new(|bean, args| {
            crate::runtime::function::MapExtensionFunction::instance().invoke(bean, args)
        }),
        "size" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => Ok(DataValue::Int(l.borrow().len() as i32)),
            _ => Err(wrong_args("size")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => Ok(DataValue::Bool(l.borrow().is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "get" => Rc::new(|bean, args| match (bean, int_arg(args, 0)) {
            (DataValue::List(l), Some(i)) => {
                let list = l.borrow();
                let idx = if i < 0 { i + list.len() as i64 } else { i };
                list.get(idx as usize)
                    .cloned()
                    .ok_or_else(|| wrong_args("get"))
            }
            _ => Err(wrong_args("get")),
        }),
        "add" => Rc::new(|bean, args| match bean {
            DataValue::List(l) => {
                l.borrow_mut().push(args.first().cloned().unwrap_or(DataValue::Null));
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
        "subList" => Rc::new(|bean, args| match (bean, int_arg(args, 0), int_arg(args, 1)) {
            (DataValue::List(l), Some(a), Some(b)) => {
                let list = l.borrow();
                let (a, b) = (a.max(0) as usize, (b.max(0) as usize).min(list.len()));
                if a > b {
                    return Err(wrong_args("subList"));
                }
                Ok(DataValue::list(list[a..b].to_vec()))
            }
            _ => Err(wrong_args("subList")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.string_value_of()))),
        _ => return None,
    };
    Some(f)
}

/// `java.util.Map` 的脚本可用方法子集。
fn map_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "size" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::Int(m.borrow().len() as i32)),
            _ => Err(wrong_args("size")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::Bool(m.borrow().is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "get" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(k)) => Ok(m.borrow().get(k).cloned().unwrap_or(DataValue::Null)),
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
            (DataValue::Map(m), Some(k)) => {
                Ok(m.borrow_mut().remove(k).unwrap_or(DataValue::Null))
            }
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
