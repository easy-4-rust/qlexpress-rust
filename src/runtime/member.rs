//! Member access (fields/methods/constructors), mirroring Java
//! `ReflectLoader`, `MethodInvokeUtils`, `MetaClass` and the `member/`
//! package — with Java reflection replaced by the explicit-registration
//! `NativeRegistry` of SPEC §4.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::data::convert::number_compare;
use crate::runtime::data::MapItemValue;
use crate::runtime::qlambda::QLambda;
use crate::runtime::value::{DataValue, NativeObject, QValue};
use crate::utils::basic_util;

/// Reference to a type where Java uses `Class<?>`, mirroring the role of
/// `MetaClass.getClz()`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClassRef {
    /// A primitive/wrapper conversion target.
    Primitive(crate::runtime::data::convert::obj_type_convertor::TargetType),
    /// A named (registered or host) type, e.g. `java.lang.String`.
    Named(String),
}

impl ClassRef {
    /// Java `Class.getName()`.
    pub fn java_name(&self) -> &str {
        match self {
            ClassRef::Primitive(target) => target.java_name(),
            ClassRef::Named(name) => name.as_str(),
        }
    }

    /// Java `Class.getSimpleName()`.
    pub fn simple_name(&self) -> &str {
        let name = self.java_name();
        name.rsplit('.').next().unwrap_or(name)
    }

    /// Resolve a Java-style type name (used by the compiler for class
    /// literals). Primitive names map to conversion targets.
    pub fn from_name(name: &str) -> ClassRef {
        use crate::runtime::data::convert::obj_type_convertor::TargetType;
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
}

/// A class literal on the operand stack, mirroring Java `MetaClass`
/// (stored inside [`DataValue::Object`]).
pub struct MetaClass {
    clz: ClassRef,
}

impl MetaClass {
    pub fn new(clz: ClassRef) -> Self {
        MetaClass { clz }
    }

    /// Java `getClz()`.
    pub fn clz(&self) -> &ClassRef {
        &self.clz
    }

    /// Wrap into a stack value (Java `new DataValue(new MetaClass(clz))`).
    pub fn into_data_value(self) -> DataValue {
        DataValue::Object(Rc::new(RefCell::new(self)))
    }
}

impl NativeObject for MetaClass {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(QLException::for_test(
            crate::exception::ql_exception::QLExceptionKind::Runtime,
            format!("method '{name}' not found on MetaClass"),
            error_codes::METHOD_NOT_FOUND,
        ))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.runtime.MetaClass"
    }
}

/// Extract the [`MetaClass`] from a stack datum (Java
/// `target instanceof MetaClass`).
pub fn as_meta_class(value: &DataValue) -> Option<ClassRef> {
    if let DataValue::Object(obj) = value {
        let borrowed = obj.borrow();
        borrowed
            .as_any()
            .downcast_ref::<MetaClass>()
            .map(|meta| meta.clz.clone())
    } else {
        None
    }
}

/// A native (host/built-in) method: `fn(receiver, args) -> result`.
pub type NativeMethod = Rc<dyn Fn(&DataValue, &[DataValue]) -> Result<DataValue, QLException>>;

/// A native constructor: `fn(args) -> instance`.
pub type NativeConstructor = Rc<dyn Fn(&[DataValue]) -> Result<DataValue, QLException>>;

/// A native field getter.
pub type NativeFieldGetter = Rc<dyn Fn(&DataValue) -> Option<DataValue>>;

/// Registered type, mirroring the reflection surface of a Java `Class`
/// (SPEC §4 `NativeType`).
#[derive(Default)]
pub struct NativeType {
    /// Canonical type name (Java `Class.getName()`).
    pub name: String,
    /// Constructor used by `NewInstanceInstruction` (Java `Constructor`).
    pub constructor: Option<NativeConstructor>,
    /// Instance methods by name (Java member `Method`s; overload
    /// resolution is by name only — see Stage-3a notes).
    pub methods: HashMap<String, NativeMethod>,
    /// Static methods by name (Java static `Method`s).
    pub static_methods: HashMap<String, NativeMethod>,
    /// Instance field getters by name (Java `Field`/getter methods).
    pub fields: HashMap<String, NativeFieldGetter>,
    /// Static fields by name (Java static `Field`s).
    pub static_fields: HashMap<String, DataValue>,
}

impl NativeType {
    pub fn named(name: impl Into<String>) -> Self {
        NativeType {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Explicit type registry replacing Java reflection (SPEC §4), mirroring
/// `ReflectLoader` + `DefaultClassSupplier` capabilities.
#[derive(Default)]
pub struct NativeRegistry {
    types: HashMap<String, NativeType>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        NativeRegistry::default()
    }

    /// Registry pre-populated with the built-in method subsets for
    /// `String`/`List`/`Map`/numbers (SPEC §4: 对齐 Java 版脚本中可用的方法子集).
    pub fn with_builtins() -> Self {
        let mut registry = NativeRegistry::new();
        registry.register_builtin_types();
        registry
    }

    pub fn register_type(&mut self, native_type: NativeType) {
        self.types.insert(native_type.name.clone(), native_type);
    }

    pub fn get_type(&self, name: &str) -> Option<&NativeType> {
        self.types.get(name)
    }

    // ---- Java ReflectLoader.loadConstructor ----

    /// Java `loadConstructor(Class, Class[])`: a registered constructor for
    /// the type (argument matching is delegated to the constructor itself).
    pub fn load_constructor(&self, clz: &ClassRef) -> Option<NativeConstructor> {
        match clz {
            ClassRef::Named(name) => self
                .types
                .get(name)
                .and_then(|t| t.constructor.as_ref().map(Rc::clone)),
            ClassRef::Primitive(_) => None,
        }
    }

    // ---- Java ReflectLoader.loadField ----

    /// Java `loadField(Object bean, String fieldName, boolean skipSecurity,
    /// ErrorReporter)`. Returns `None` when the field does not exist.
    pub fn load_field(&self, bean: &DataValue, field_name: &str) -> Option<QValue> {
        match bean {
            // Java: array length
            DataValue::Array(arr) if field_name == basic_util::LENGTH => {
                Some(QValue::Data(DataValue::Int(arr.borrow().len() as i32)))
            }
            // Java: list length
            DataValue::List(list) if field_name == basic_util::LENGTH => {
                Some(QValue::Data(DataValue::Int(list.borrow().len() as i32)))
            }
            // Java: Map → MapItemValue
            DataValue::Map(map) => Some(QValue::Left(Rc::new(RefCell::new(
                MapItemValue::new(Rc::clone(map), DataValue::Str(field_name.to_string())),
            )))),
            DataValue::Object(obj) => {
                // Java MetaClass branch: `.class` and static fields.
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
                            // Java returns the Class object itself; the
                            // closest stack value is the MetaClass datum.
                            return Some(QValue::Data(bean.clone()));
                        }
                        if let ClassRef::Named(name) = clz {
                            if let Some(value) = self
                                .types
                                .get(&name)
                                .and_then(|t| t.static_fields.get(field_name))
                            {
                                return Some(QValue::Data(value.clone()));
                            }
                        }
                        None
                    }
                    None => obj.borrow().get_field(field_name).map(QValue::Data),
                }
            }
            _ => {
                // Registered instance fields (by Java type name).
                let type_name = bean.data_type_name();
                self.types
                    .get(type_name)
                    .and_then(|t| t.fields.get(field_name))
                    .and_then(|getter| getter(bean))
                    .map(QValue::Data)
            }
        }
    }

    // ---- Java ReflectLoader.loadMethod + MethodHandler ----

    /// Resolve a callable method on `bean` by name. Returns `None` when no
    /// such method exists (Java `loadMethod` returning `null`).
    pub fn resolve_method(&self, bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
        // MetaClass receiver → static method (Java `isStaticMethod`).
        if let Some(meta) = as_meta_class(bean) {
            if let ClassRef::Named(name) = meta {
                return self
                    .types
                    .get(&name)
                    .and_then(|t| t.static_methods.get(method_name).map(Rc::clone));
            }
            return None;
        }
        if let Some(method) = builtin_method(bean, method_name) {
            return Some(method);
        }
        let type_name = bean.data_type_name();
        if let Some(method) = self
            .types
            .get(type_name)
            .and_then(|t| t.methods.get(method_name).map(Rc::clone))
        {
            return Some(method);
        }
        None
    }

    fn register_builtin_types(&mut self) {
        // Marker types so `ClassSupplier`-style lookups and future host
        // overrides have anchor points; the dispatch itself is in
        // `builtin_method`.
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

/// Invoke a resolved method, mirroring Java `MethodInvokeUtils.invokeIMethod`
/// (conversion errors/inner errors propagate as `QLException`, like Java
/// rethrowing `QLRuntimeException`).
pub fn invoke_native_method(
    bean: &DataValue,
    method: &NativeMethod,
    params: &[DataValue],
) -> Result<QValue, QLException> {
    method(bean, params).map(QValue::Data)
}

/// Java `MethodInvokeUtils.findQLambdaInstance`: a lambda stored in a Map
/// under the method name is callable as a "method".
fn find_q_lambda_instance(bean: &DataValue, method_name: &str) -> Option<Rc<QLambda>> {
    if let DataValue::Map(map) = bean {
        if let Some(DataValue::Lambda(lambda)) =
            map.borrow().get(&DataValue::Str(method_name.to_string()))
        {
            return Some(Rc::clone(lambda));
        }
    }
    None
}

/// Java `MethodInvokeUtils.findMethodAndInvoke`.
pub fn find_method_and_invoke(
    bean: &DataValue,
    method_name: &str,
    params: &[DataValue],
    registry: &NativeRegistry,
    error_reporter: &dyn ErrorReporter,
) -> Result<QValue, QLException> {
    if let Some(method) = registry.resolve_method(bean, method_name) {
        return invoke_native_method(bean, &method, params);
    }
    // Host object dynamic dispatch (NativeObject::call_method).
    if let DataValue::Object(obj) = bean {
        if as_meta_class(bean).is_none() {
            let result = obj.borrow_mut().call_method(method_name, params)?;
            return Ok(QValue::Data(result));
        }
    }
    if let Some(q_lambda) = find_q_lambda_instance(bean, method_name) {
        let q_result = q_lambda.call(params)?;
        return Ok(q_result.value().into());
    }
    let params_render = params
        .iter()
        .map(DataValue::string_value_of)
        .collect::<Vec<_>>()
        .join(", ");
    Err(error_reporter.report_format(
        error_codes::METHOD_NOT_FOUND,
        error_codes::error_msg(error_codes::METHOD_NOT_FOUND),
        &[method_name.to_string(), format!("[{params_render}]")],
    ))
}

// ---------------------------------------------------------------------------
// Built-in method subsets (SPEC §4: String/List/Map/数组 常用方法).
// ---------------------------------------------------------------------------

fn int_arg(args: &[DataValue], i: usize) -> Option<i64> {
    args.get(i).and_then(|v| {
        if v.is_number() {
            Some(crate::runtime::data::convert::to_i64(v))
        } else {
            None
        }
    })
}

fn str_arg(args: &[DataValue], i: usize) -> Option<String> {
    args.get(i).map(|v| v.string_value_of())
}

fn wrong_args(method: &str) -> QLException {
    QLException::for_test(
        crate::exception::ql_exception::QLExceptionKind::Runtime,
        format!("invoke method '{method}' with wrong arguments"),
        error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
    )
}

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
                let ord = number_compare(bean, other)
                    .unwrap_or(std::cmp::Ordering::Equal);
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

