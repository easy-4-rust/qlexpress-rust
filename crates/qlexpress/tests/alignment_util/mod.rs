//! Stage 6 对齐测试公共工具:复刻 Java `TestSuiteRunner` 的
//! assert / assertFalse / assertErrorCode / println 四个测试函数,
//! 以及脚本执行的便捷封装。
//!
//! 对应 Java: com.alibaba.qlexpress4.TestSuiteRunner 的内部类
//! `AssertFunction` / `AssertFalseFunction` / `AssertErrorCodeFunction` / `PrintFunction`。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类。
#![allow(clippy::result_large_err)]
// 本文件会分别编译进多个 integration-test crate；每个 crate 只使用其中
// 一部分公共夹具，因此按单个 crate 检查时其余夹具必然显示为 dead_code。
#![allow(dead_code)]

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;

use std::rc::Rc;

use qlexpress::class_supplier::ClassSupplier;
use qlexpress::exception::error_codes;
use qlexpress::exception::ql_exception::{QLException, QLExceptionKind};
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::class_ref::ClassRef;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::{NativeConstructorCandidate, NativeType};
use qlexpress::runtime::opaque_native_object::OpaqueNativeObject;
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

/// 测试用类型供应器:模拟 Java `DefaultClassSupplier`(`Class.forName`)
/// 对常见 JDK 类的解析。Rust 版无 JVM 类路径(架构性偏差,无反射),
/// 测试中用已知 JDK 类名白名单近似,使 `String x;`、`instanceof ArrayList`
/// 等脚本语法可解析。
pub struct JdkClassSupplier;

/// 常见 JDK 类(包 → 简单名),模拟 `Class.forName` 可加载的类集合。
const JDK_CLASSES: &[(&str, &str)] = &[
    ("java.lang", "String"),
    ("java.lang", "Object"),
    ("java.lang", "Integer"),
    ("java.lang", "Long"),
    ("java.lang", "Double"),
    ("java.lang", "Float"),
    ("java.lang", "Short"),
    ("java.lang", "Byte"),
    ("java.lang", "Character"),
    ("java.lang", "Boolean"),
    ("java.lang", "Number"),
    ("java.lang", "Math"),
    ("java.lang", "StringBuilder"),
    ("java.lang", "StringBuffer"),
    ("java.lang", "Exception"),
    ("java.lang", "RuntimeException"),
    ("java.lang", "NullPointerException"),
    ("java.lang", "IllegalArgumentException"),
    ("java.lang", "IllegalStateException"),
    ("java.lang", "ClassCastException"),
    ("java.lang", "ArithmeticException"),
    ("java.lang", "IndexOutOfBoundsException"),
    ("java.lang", "ArrayIndexOutOfBoundsException"),
    ("java.lang", "UnsupportedOperationException"),
    ("java.lang", "Comparable"),
    ("java.lang", "Runnable"),
    ("java.io", "Serializable"),
    ("java.math", "BigDecimal"),
    ("java.math", "BigInteger"),
    ("java.util", "List"),
    ("java.util", "ArrayList"),
    ("java.util", "LinkedList"),
    ("java.util", "Map"),
    ("java.util", "HashMap"),
    ("java.util", "LinkedHashMap"),
    ("java.util", "TreeMap"),
    ("java.util", "Set"),
    ("java.util", "HashSet"),
    ("java.util", "LinkedHashSet"),
    ("java.util", "Arrays"),
    ("java.util", "Objects"),
    ("java.util", "Collections"),
    ("java.util", "Date"),
    ("java.util.stream", "Collectors"),
    ("java.util.stream", "Stream"),
    ("java.util.function", "Supplier"),
    ("java.util.function", "Consumer"),
    ("java.util.function", "Function"),
    ("java.util.function", "Predicate"),
    ("com.alibaba.qlexpress4.inport", "Person"),
    ("com.alibaba.qlexpress4.inport", "Sample"),
    ("com.alibaba.qlexpress4.inport", "Sample1"),
    ("com.alibaba.qlexpress4.inport", "MyHome"),
    ("com.alibaba.qlexpress4.inport", "MyDesk"),
    (
        "com.alibaba.qlexpress4.test.constructor",
        "HelloConstructor",
    ),
    ("com.alibaba.qlexpress4.test.constructor", "HelloParent"),
    ("com.alibaba.qlexpress4.test.constructor", "HelloChild"),
    (
        "com.alibaba.qlexpress4.test.lambda",
        "UserFunctionalInterface",
    ),
    ("com.alibaba.qlexpress4.test.property", "Sample"),
    ("com.alibaba.qlexpress4.test.property", "SampleEnum"),
    ("com.alibaba.qlexpress4.test.property", "SomeInter"),
    ("com.alibaba.qlexpress4.test.property", "Parent"),
    ("com.alibaba.qlexpress4.test.property", "SampleSet"),
    ("com.alibaba.qlexpress4.test.property", "SampleForPrivate"),
    ("com.alibaba.qlexpress4.test.property", "TestEnum"),
    ("com.alibaba.qlexpress4.test.method", "TestChild"),
    ("com.alibaba.qlexpress4.test.method", "TestParent"),
    ("com.alibaba.qlexpress4.test.method", "InterWithDefault"),
    ("com.alibaba.qlexpress4.test.stream", "STObject"),
    ("com.alibaba.fastjson2", "JSON"),
    ("com.alibaba.fastjson2", "JSONObject"),
];

impl ClassSupplier for JdkClassSupplier {
    fn load_cls(&self, qualified_name: &str) -> Option<String> {
        let (pack, simple) = qualified_name.rsplit_once('.')?;
        if JDK_CLASSES.contains(&(pack, simple)) {
            Some(qualified_name.to_string())
        } else {
            None
        }
    }
}

/// 带 JDK 类型供应器与开放安全策略的 InitOptions,对应 Java
/// `TestSuiteRunner.handleFile` 中的
/// `InitOptions.builder().securityStrategy(QLSecurityStrategy.open()).build()`。
pub fn jdk_init_options() -> InitOptions {
    InitOptions::builder()
        .class_supplier(Rc::new(JdkClassSupplier))
        .security_strategy(QLSecurityStrategy::open())
        .build()
}

/// Java `UserDefineException(message)`:错误码 `BIZ_EXCEPTION`。
pub fn biz_error(message: impl Into<String>) -> QLException {
    QLException::for_test(
        QLExceptionKind::Runtime,
        message.into(),
        error_codes::BIZ_EXCEPTION,
    )
}

fn wrap_assert_message(q_context: &dyn QContext, message: &str) -> String {
    let test_path = q_context
        .attachment()
        .get("TEST_PATH")
        .map(DataValue::string_value_of)
        .unwrap_or_else(|| "null".to_string());
    format!("{test_path}: {message}")
}

/// 对应 Java `TestSuiteRunner.AssertFunction`:
/// `assert(bool)` / `assert(bool, message)`,为 false 或 null 时抛
/// `UserDefineException`(BIZ_EXCEPTION)。
fn assert_function(ctx: &mut dyn QContext, params: &Parameters) -> Result<DataValue, QLException> {
    match params.size() {
        1 => match params.get_value(0) {
            DataValue::Bool(true) => Ok(DataValue::Null),
            _ => Err(biz_error(wrap_assert_message(ctx, "assert fail"))),
        },
        2 => match params.get_value(0) {
            DataValue::Bool(true) => Ok(DataValue::Null),
            _ => match params.get_value(1) {
                DataValue::Str(msg) => {
                    Err(biz_error(wrap_assert_message(ctx, &msg.to_string_lossy())))
                }
                _ => Err(biz_error(wrap_assert_message(ctx, "assert fail"))),
            },
        },
        n => Err(biz_error(format!("invalid parameter size: {n}"))),
    }
}

/// 对应 Java `TestSuiteRunner.AssertFalseFunction`。
fn assert_false_function(
    ctx: &mut dyn QContext,
    params: &Parameters,
) -> Result<DataValue, QLException> {
    match params.size() {
        1 => match params.get_value(0) {
            DataValue::Bool(false) => Ok(DataValue::Null),
            _ => Err(biz_error(wrap_assert_message(ctx, "assert fail"))),
        },
        2 => match params.get_value(0) {
            DataValue::Bool(false) => Ok(DataValue::Null),
            _ => match params.get_value(1) {
                DataValue::Str(msg) => {
                    Err(biz_error(wrap_assert_message(ctx, &msg.to_string_lossy())))
                }
                _ => Err(biz_error(wrap_assert_message(ctx, "assert fail"))),
            },
        },
        n => Err(biz_error(format!("invalid parameter size: {n}"))),
    }
}

/// 对应 Java `TestSuiteRunner.AssertErrorCodeFunction`:
/// `assertErrorCode(() -> ..., "ERR_CODE")`,调用 lambda 并校验抛出的
/// `QLException.errorCode` 与期望一致。
fn assert_error_code_function(
    _ctx: &mut dyn QContext,
    params: &Parameters,
) -> Result<DataValue, QLException> {
    if params.size() != 2 {
        return Err(biz_error(format!(
            "invalid pSize:{}, expected 2 parameters",
            params.size()
        )));
    }
    let lambda = match params.get_value(0) {
        DataValue::Lambda(lambda) => lambda,
        other => {
            return Err(biz_error(format!(
                "assertErrorCode 第一个参数必须是 lambda,实际为 {other:?}"
            )))
        }
    };
    let expected_code = match params.get_value(1) {
        DataValue::Str(code) => code,
        other => {
            return Err(biz_error(format!(
                "assertErrorCode 第二个参数必须是字符串错误码,实际为 {other:?}"
            )))
        }
    };
    match lambda.call(&[]) {
        Ok(_) => Err(biz_error(format!(
            "expect error codes:{expected_code}, but end normally"
        ))),
        Err(err) if expected_code.as_str() == Some(err.error_code()) => Ok(DataValue::Null),
        Err(err) => Err(biz_error(format!(
            "expect error code {expected_code}, but got {}: {}",
            err.error_code(),
            err.reason()
        ))),
    }
}

/// 对应 Java `TestSuiteRunner.PrintFunction`(丢弃输出,仅消费参数)。
fn print_function(_ctx: &mut dyn QContext, _params: &Parameters) -> Result<DataValue, QLException> {
    Ok(DataValue::Null)
}

/// Java 测试夹具 `com.alibaba.qlexpress4.inport.Person` 的 Rust 对等对象。
struct TestPerson {
    age: i64,
}

/// Java 测试夹具 `HelloConstructor` 的 Rust 对等对象，仅暴露 Java 中的
/// `public int flag`，用于验证构造器重载选择结果。
struct FlagObject {
    flag: i32,
}

/// Java 测试夹具 `test.property.Sample` 的 Rust 等价对象。
struct PropertySampleObject {
    count: i32,
}

/// Java `SampleEnum.NORMAL` / `UN_SUPPORT` 的 Rust 测试夹具实例。
struct SampleEnumObject;

/// Java 测试夹具 `property.Parent`：只承载本套件实际使用的出生日期属性。
struct ParentObject {
    birth: DataValue,
    lock_status: i32,
    lock_status2: DataValue,
}

/// Java 测试夹具 `property.SampleSet` 的公开 `count` 字段。
struct CountObject {
    type_name: &'static str,
    count: i32,
}

/// Java 测试夹具 `method.TestChild` 的方法分派对象。
struct TestChildObject;

/// Java 测试夹具 `stream.STObject` 的不可变 payload。
struct StreamTestObject {
    payload: DataValue,
}

/// Java 分类 Map 目标 `MyHome` / `MyDesk` 的字段容器。
struct ClassifiedObject {
    type_name: &'static str,
    fields: HashMap<String, DataValue>,
}

/// Java fastjson2 `JSONObject`：测试所需的字符串键 `put/get` 有序对象。
struct JsonObject {
    entries: qlexpress::runtime::data::index_map::IndexMap,
}

impl NativeObject for SampleEnumObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "testField").then_some(DataValue::Int(10))
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(biz_error(format!("SampleEnum method not found: {name}")))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.property.SampleEnum"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for ParentObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        match name {
            "birth" => Some(self.birth.clone()),
            "lockStatus" => Some(DataValue::Int(self.lock_status)),
            "lockStatus2" => Some(self.lock_status2.clone()),
            _ => None,
        }
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        match (name, value) {
            ("birth", value) if value.is_null() || matches!(value, DataValue::Str(_)) => {
                self.birth = value.clone();
                true
            }
            ("lockStatus", value) if value.is_number() => {
                self.lock_status = qlexpress::runtime::data::convert::to_i32(value);
                true
            }
            ("lockStatus2", value) if value.is_null() || value.is_number() => {
                self.lock_status2 = value.clone();
                true
            }
            _ => false,
        }
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("getBirth", []) => Ok(self.birth.clone()),
            ("setBirth", [value]) if value.is_null() || matches!(value, DataValue::Str(_)) => {
                self.birth = value.clone();
                Ok(DataValue::Null)
            }
            _ => Err(biz_error(format!("Parent method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.property.Parent"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for CountObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "count").then_some(DataValue::Int(self.count))
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        if name == "count" && value.is_number() {
            self.count = qlexpress::runtime::data::convert::to_i32(value);
            true
        } else {
            false
        }
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(biz_error(format!(
            "{} method not found: {name}",
            self.type_name
        )))
    }

    fn native_type_name(&self) -> &str {
        self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for TestChildObject {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("get10", []) => Ok(DataValue::Int(10)),
            ("get10", [DataValue::Str(_)]) => Ok(DataValue::Int(11)),
            ("get1", []) => Ok(DataValue::Int(1)),
            ("get100", []) => Ok(DataValue::Int(100)),
            _ => Err(biz_error(format!("TestChild method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.method.TestChild"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for StreamTestObject {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("getPayload", []) => Ok(self.payload.clone()),
            _ => Err(biz_error(format!("STObject method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.stream.STObject"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for ClassifiedObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        if matches!(
            name,
            "sofa" | "chair" | "myDesk" | "bed" | "book1" | "book2"
        ) {
            Some(self.fields.get(name).cloned().unwrap_or(DataValue::Null))
        } else {
            None
        }
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        if matches!(name, "sofa" | "chair" | "myDesk" | "book1" | "book2") {
            self.fields.insert(name.to_string(), value.clone());
            true
        } else {
            false
        }
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        let field = match (name, args) {
            ("getSofa", []) => "sofa",
            ("getChair", []) => "chair",
            ("getMyDesk", []) => "myDesk",
            ("getBed", []) => "bed",
            ("getBook1", []) => "book1",
            ("getBook2", []) => "book2",
            ("setSofa", [value]) => {
                self.fields.insert("sofa".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setChair", [value]) => {
                self.fields.insert("chair".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setMyDesk", [value]) => {
                self.fields.insert("myDesk".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setBook1", [value]) => {
                self.fields.insert("book1".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setBook2", [value]) => {
                self.fields.insert("book2".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            _ => {
                return Err(biz_error(format!(
                    "{} method not found: {name}",
                    self.type_name
                )))
            }
        };
        Ok(self.fields.get(field).cloned().unwrap_or(DataValue::Null))
    }

    fn native_type_name(&self) -> &str {
        self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for JsonObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        self.entries.get(&DataValue::string(name)).cloned()
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        self.entries.insert(DataValue::string(name), value.clone());
        true
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("put", [DataValue::Str(key), value]) => Ok(self
                .entries
                .insert(DataValue::Str(key.clone()), value.clone())
                .unwrap_or(DataValue::Null)),
            ("get", [DataValue::Str(key)]) => Ok(self
                .entries
                .get(&DataValue::Str(key.clone()))
                .cloned()
                .unwrap_or(DataValue::Null)),
            _ => Err(biz_error(format!("JSONObject method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.fastjson2.JSONObject"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for PropertySampleObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "count").then_some(DataValue::Int(self.count))
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        if name == "count" && value.is_number() {
            self.count = qlexpress::runtime::data::convert::to_i32(value);
            return true;
        }
        false
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("getCount", []) => Ok(DataValue::Int(self.count)),
            ("setCount", [value]) if value.is_number() => {
                self.count = qlexpress::runtime::data::convert::to_i32(value);
                Ok(DataValue::Null)
            }
            _ => Err(biz_error(format!("Sample method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.property.Sample"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for FlagObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "flag").then_some(DataValue::Int(self.flag))
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(biz_error(format!(
            "HelloConstructor method not found: {name}"
        )))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.constructor.HelloConstructor"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn flag_object(flag: i32) -> DataValue {
    DataValue::Object(Rc::new(RefCell::new(FlagObject { flag })))
}

fn named_class(name: &str) -> ClassRef {
    ClassRef::Named(name.to_string())
}

fn constructor_candidate(
    parameter_types: Vec<ClassRef>,
    var_args: bool,
    flag: i32,
) -> NativeConstructorCandidate {
    NativeConstructorCandidate::new(
        parameter_types,
        var_args,
        Rc::new(move |_args| Ok(flag_object(flag))),
    )
}

impl NativeObject for TestPerson {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "age").then_some(DataValue::Long(self.age))
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        if name == "compareTo" {
            let Some(DataValue::Object(other)) = args.first() else {
                return Err(biz_error("Person.compareTo expects Person"));
            };
            let borrowed = other.borrow();
            let Some(other) = borrowed.as_any().downcast_ref::<TestPerson>() else {
                return Err(biz_error("Person.compareTo expects Person"));
            };
            return Ok(DataValue::Int(match self.age.cmp(&other.age) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }));
        }
        Err(biz_error(format!("Person method not found: {name}")))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.inport.Person"
    }

    fn is_comparable(&self) -> bool {
        true
    }

    fn compare_to(&self, other: &dyn NativeObject) -> Option<Ordering> {
        other
            .as_any()
            .downcast_ref::<TestPerson>()
            .map(|other| self.age.cmp(&other.age))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 对应 Java `TestSuiteRunner.prepareRunner`:注册四个测试函数,
/// 并使用开放安全策略 + JDK 类型供应器(同 `handleFile`)。
pub fn suite_runner() -> Express4Runner {
    suite_runner_with_init_options(jdk_init_options())
}

/// 使用指定初始化选项创建套件 Runner，同时注册 Java 套件的四个函数。
///
/// 对应 Java `TestSuiteRunner#prepareRunner(InitOptions)`；供 debug 与
/// 自定义初始化策略测试复用相同宿主装配逻辑。
pub fn suite_runner_with_init_options(init_options: InitOptions) -> Express4Runner {
    let allow_private_access = init_options.is_allow_private_access();
    let mut runner = Express4Runner::with_init_options(init_options);
    let mut person = NativeType::named("com.alibaba.qlexpress4.inport.Person");
    person.constructor = Some(Rc::new(|args| {
        let [age] = args else {
            return Err(biz_error("Person constructor expects one age"));
        };
        if !age.is_number() {
            return Err(biz_error("Person constructor expects numeric age"));
        }
        let object: Rc<RefCell<dyn NativeObject>> = Rc::new(RefCell::new(TestPerson {
            age: qlexpress::runtime::data::convert::to_i64(age),
        }));
        Ok(DataValue::Object(object))
    }));
    runner.register_native_type(person);
    // 对应 Java testsuite import fixture 中的 `Sample.value == 1` 与
    // `Sample1.value == 10`。Java 通过反射读取 public static 字段；Rust
    // 必须显式注册同名 capability，避免把测试夹具变成隐式反射白名单。
    let mut sample = NativeType::named("com.alibaba.qlexpress4.inport.Sample");
    sample
        .static_fields
        .insert("value".to_string(), DataValue::Int(1));
    runner.register_native_type(sample);
    let mut sample1 = NativeType::named("com.alibaba.qlexpress4.inport.Sample1");
    sample1
        .static_fields
        .insert("value".to_string(), DataValue::Int(10));
    runner.register_native_type(sample1);
    // Java Map 字面量的 `@class` 分类实例化：未知字段由
    // NewFilledInstanceInstruction 忽略，已登记字段经显式 NativeObject 写入。
    for (type_name, methods) in [
        (
            "com.alibaba.qlexpress4.inport.MyHome",
            [
                "getSofa",
                "setSofa",
                "getChair",
                "setChair",
                "getMyDesk",
                "setMyDesk",
                "getBed",
                "",
            ],
        ),
        (
            "com.alibaba.qlexpress4.inport.MyDesk",
            [
                "getBook1", "setBook1", "getBook2", "setBook2", "", "", "", "",
            ],
        ),
    ] {
        let mut classified = NativeType::named(type_name);
        classified.constructor = Some(Rc::new(move |args| match args {
            [] => Ok(DataValue::Object(Rc::new(RefCell::new(ClassifiedObject {
                type_name,
                fields: HashMap::new(),
            })))),
            _ => Err(biz_error(format!("{type_name} constructor arguments"))),
        }));
        for method in methods.into_iter().filter(|method| !method.is_empty()) {
            classified.methods.insert(
                method.to_string(),
                Rc::new(move |bean, args| match bean {
                    DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                    _ => Err(biz_error(format!("{method} receiver"))),
                }),
            );
        }
        if type_name == "com.alibaba.qlexpress4.inport.MyHome" {
            // Java MyHome.bed 有 getter 但没有 setter；分类构造命中它时不是
            // “未知字段可忽略”，而是必须报 INVALID_ASSIGNMENT。
            classified.fields.insert(
                "bed".to_string(),
                Rc::new(|bean| match bean {
                    DataValue::Object(object) => object.borrow().get_field("bed"),
                    _ => None,
                }),
            );
        }
        runner.register_native_type(classified);
    }
    // 对应 Java `HelloParent`/`HelloChild`/`HelloConstructor` 测试夹具的
    // 全部公开构造器。候选签名交给 NativeRegistry 按 Java 可赋值性与
    // 可变参数优先级选择，不能以 if/else 伪造重载结果。
    let parent_name = "com.alibaba.qlexpress4.test.constructor.HelloParent";
    let child_name = "com.alibaba.qlexpress4.test.constructor.HelloChild";
    let hello_name = "com.alibaba.qlexpress4.test.constructor.HelloConstructor";
    let mut parent = NativeType::named(parent_name);
    parent.constructor = Some(Rc::new(move |args| match args {
        [] => Ok(OpaqueNativeObject::new(parent_name).into_data_value()),
        _ => Err(biz_error("HelloParent constructor arguments")),
    }));
    runner.register_native_type(parent);
    let mut child = NativeType::named(child_name);
    child.supertypes.push(parent_name.to_string());
    child.constructor = Some(Rc::new(move |args| match args {
        [] => Ok(OpaqueNativeObject::new(child_name).into_data_value()),
        _ => Err(biz_error("HelloChild constructor arguments")),
    }));
    runner.register_native_type(child);
    runner.register_native_type(NativeType::interface("java.lang.Runnable", ["run"]));
    runner.register_native_type(NativeType::interface(
        "com.alibaba.qlexpress4.test.lambda.UserFunctionalInterface",
        ["lala"],
    ));
    let mut hello = NativeType::named(hello_name);
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class(parent_name)],
        false,
        0,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class(child_name)],
        false,
        1,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![ClassRef::array_of(named_class("java.lang.String"))],
        true,
        2,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class("java.lang.String")],
        false,
        3,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class(child_name), named_class("java.lang.Runnable")],
        false,
        4,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![
            named_class(parent_name),
            named_class("com.alibaba.qlexpress4.runtime.QLambda"),
        ],
        false,
        5,
    ));
    hello.fields.insert(
        "flag".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("flag"),
            _ => None,
        }),
    );
    runner.register_native_type(hello);
    let sample_property_name = "com.alibaba.qlexpress4.test.property.Sample";
    let mut property_sample = NativeType::named(sample_property_name);
    property_sample.constructor = Some(Rc::new(|args| match args {
        [value] if value.is_number() => Ok(DataValue::Object(Rc::new(RefCell::new(
            PropertySampleObject {
                count: qlexpress::runtime::data::convert::to_i32(value),
            },
        )))),
        _ => Err(biz_error("Sample constructor arguments")),
    }));
    property_sample.methods.insert(
        "getCount".to_string(),
        Rc::new(|bean, args| match bean {
            DataValue::Object(object) => object.borrow_mut().call_method("getCount", args),
            _ => Err(biz_error("Sample.getCount receiver")),
        }),
    );
    property_sample.methods.insert(
        "setCount".to_string(),
        Rc::new(|bean, args| match bean {
            DataValue::Object(object) => object.borrow_mut().call_method("setCount", args),
            _ => Err(biz_error("Sample.setCount receiver")),
        }),
    );
    // Java `Sample.count` 是 private，但 ReflectLoader 通过 getCount/setCount
    // 暴露同名属性；Rust 将该 JavaBean 属性显式登记为可读写字段能力。
    property_sample.fields.insert(
        "count".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("count"),
            _ => None,
        }),
    );
    property_sample.field_setters.insert(
        "count".to_string(),
        Rc::new(|bean, value| match bean {
            DataValue::Object(object) => object.borrow_mut().set_field("count", value),
            _ => false,
        }),
    );
    runner.register_native_type(property_sample);
    let sample_enum_name = "com.alibaba.qlexpress4.test.property.SampleEnum";
    let normal_enum = DataValue::Object(Rc::new(RefCell::new(SampleEnumObject)));
    let unsupported_enum = DataValue::Object(Rc::new(RefCell::new(SampleEnumObject)));
    let mut sample_enum = NativeType::named(sample_enum_name);
    sample_enum
        .static_fields
        .insert("NORMAL".to_string(), normal_enum);
    sample_enum
        .static_fields
        .insert("UN_SUPPORT".to_string(), unsupported_enum);
    sample_enum
        .static_fields
        .insert("testStaticField".to_string(), DataValue::Int(1000));
    sample_enum.fields.insert(
        "testField".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("testField"),
            _ => None,
        }),
    );
    sample_enum.methods.insert(
        "equals".to_string(),
        Rc::new(|bean, args| match (bean, args) {
            (DataValue::Object(left), [DataValue::Object(right)]) => {
                Ok(DataValue::Bool(Rc::ptr_eq(left, right)))
            }
            (DataValue::Object(_), [_]) => Ok(DataValue::Bool(false)),
            _ => Err(biz_error("SampleEnum.equals receiver")),
        }),
    );
    runner.register_native_type(sample_enum);
    let mut some_inter = NativeType::named("com.alibaba.qlexpress4.test.property.SomeInter");
    some_inter
        .static_fields
        .insert("INTER_CONST_1".to_string(), DataValue::string("test1"));
    runner.register_native_type(some_inter);
    // Java `Parent.birth` 同时可经 public field 与 JavaBean getter/setter
    // 访问；这里只登记原脚本需要的同一能力面。
    let parent_property_name = "com.alibaba.qlexpress4.test.property.Parent";
    let mut parent_property = NativeType::named(parent_property_name);
    parent_property.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(ParentObject {
            birth: DataValue::string("2022-01-01"),
            lock_status: 0,
            lock_status2: DataValue::Null,
        })))),
        _ => Err(biz_error("Parent constructor arguments")),
    }));
    parent_property.fields.insert(
        "birth".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("birth"),
            _ => None,
        }),
    );
    parent_property.field_setters.insert(
        "birth".to_string(),
        Rc::new(|bean, value| match bean {
            DataValue::Object(object) => object.borrow_mut().set_field("birth", value),
            _ => false,
        }),
    );
    for field_name in ["lockStatus", "lockStatus2"] {
        parent_property.fields.insert(
            field_name.to_string(),
            Rc::new(move |bean| match bean {
                DataValue::Object(object) => object.borrow().get_field(field_name),
                _ => None,
            }),
        );
        parent_property.field_setters.insert(
            field_name.to_string(),
            Rc::new(move |bean, value| match bean {
                DataValue::Object(object) => object.borrow_mut().set_field(field_name, value),
                _ => false,
            }),
        );
    }
    for method in ["getBirth", "setBirth"] {
        parent_property.methods.insert(
            method.to_string(),
            Rc::new(move |bean, args| match bean {
                DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                _ => Err(biz_error("Parent method receiver")),
            }),
        );
    }
    runner.register_native_type(parent_property);
    // fastjson2 JSONObject 保留自身运行时类型，同时显式暴露 Java Map 风格
    // 的 `put/get`，避免将声明类型静默降级为 LinkedHashMap。
    let mut json_object = NativeType::named("com.alibaba.fastjson2.JSONObject");
    json_object.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(JsonObject {
            entries: qlexpress::runtime::data::index_map::IndexMap::new(),
        })))),
        _ => Err(biz_error("JSONObject constructor arguments")),
    }));
    for method in ["put", "get"] {
        json_object.methods.insert(
            method.to_string(),
            Rc::new(move |bean, args| match bean {
                DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                _ => Err(biz_error("JSONObject receiver")),
            }),
        );
    }
    runner.register_native_type(json_object);
    let mut json = NativeType::named("com.alibaba.fastjson2.JSON");
    json.static_methods.insert(
        "toJSONString".to_string(),
        Rc::new(|_bean, args| match args {
            [value] => Ok(DataValue::string(value.string_value_of())),
            _ => Err(biz_error("JSON.toJSONString arguments")),
        }),
    );
    runner.register_native_type(json);
    let mut sample_set = NativeType::named("com.alibaba.qlexpress4.test.property.SampleSet");
    sample_set.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(CountObject {
            type_name: "com.alibaba.qlexpress4.test.property.SampleSet",
            count: 0,
        })))),
        _ => Err(biz_error("SampleSet constructor arguments")),
    }));
    sample_set.fields.insert(
        "count".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("count"),
            _ => None,
        }),
    );
    sample_set.field_setters.insert(
        "count".to_string(),
        Rc::new(|bean, value| match bean {
            DataValue::Object(object) => object.borrow_mut().set_field("count", value),
            _ => false,
        }),
    );
    runner.register_native_type(sample_set);
    // 私有成员不是 Rust 反射能力；仅当 Java 对应选项打开时显式登记，未打开
    // 的 runner 不能看见该字段，保持 `FIELD_NOT_FOUND`。
    let mut private_sample =
        NativeType::named("com.alibaba.qlexpress4.test.property.SampleForPrivate");
    private_sample.constructor = Some(Rc::new(|args| match args {
        [value] if value.is_number() => Ok(DataValue::Object(Rc::new(RefCell::new(CountObject {
            type_name: "com.alibaba.qlexpress4.test.property.SampleForPrivate",
            count: qlexpress::runtime::data::convert::to_i32(value),
        })))),
        _ => Err(biz_error("SampleForPrivate constructor arguments")),
    }));
    if allow_private_access {
        private_sample.fields.insert(
            "count".to_string(),
            Rc::new(|bean| match bean {
                DataValue::Object(object) => object.borrow().get_field("count"),
                _ => None,
            }),
        );
        private_sample.field_setters.insert(
            "count".to_string(),
            Rc::new(|bean, value| match bean {
                DataValue::Object(object) => object.borrow_mut().set_field("count", value),
                _ => false,
            }),
        );
    }
    runner.register_native_type(private_sample);
    let mut test_enum = NativeType::named("com.alibaba.qlexpress4.test.property.TestEnum");
    test_enum.static_fields.insert(
        "SKT".to_string(),
        DataValue::Object(Rc::new(RefCell::new(CountObject {
            type_name: "com.alibaba.qlexpress4.test.property.TestEnum",
            count: -1,
        }))),
    );
    test_enum.fields.insert(
        "value".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("count"),
            _ => None,
        }),
    );
    runner.register_native_type(test_enum);
    let child_type_name = "com.alibaba.qlexpress4.test.method.TestChild";
    let mut test_child = NativeType::named(child_type_name);
    test_child.supertypes.extend([
        "com.alibaba.qlexpress4.test.method.TestParent".to_string(),
        "com.alibaba.qlexpress4.test.method.InterWithDefault".to_string(),
    ]);
    test_child.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(TestChildObject)))),
        _ => Err(biz_error("TestChild constructor arguments")),
    }));
    for method in ["get10", "get1", "get100"] {
        test_child.methods.insert(
            method.to_string(),
            Rc::new(move |bean, args| match bean {
                DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                _ => Err(biz_error("TestChild method receiver")),
            }),
        );
    }
    runner.register_native_type(test_child);
    let mut stream_test_type = NativeType::named("com.alibaba.qlexpress4.test.stream.STObject");
    stream_test_type.constructor = Some(Rc::new(|args| match args {
        [DataValue::Str(payload)] => {
            Ok(DataValue::Object(Rc::new(RefCell::new(StreamTestObject {
                payload: DataValue::Str(payload.clone()),
            }))))
        }
        _ => Err(biz_error("STObject constructor arguments")),
    }));
    stream_test_type.methods.insert(
        "getPayload".to_string(),
        Rc::new(|bean, args| match bean {
            DataValue::Object(object) => object.borrow_mut().call_method("getPayload", args),
            _ => Err(biz_error("STObject.getPayload receiver")),
        }),
    );
    runner.register_native_type(stream_test_type);
    runner.add_function("assert", assert_function);
    runner.add_function("assertFalse", assert_false_function);
    runner.add_function("assertErrorCode", assert_error_code_function);
    runner.add_function("println", print_function);
    runner
}

/// 默认选项执行脚本,返回结果。
pub fn run_script(script: &str) -> Result<DataValue, QLException> {
    suite_runner()
        .execute(script, HashMap::new(), &QLOptions::builder().build())
        .map(|r| r.into_result())
}

/// 指定选项执行脚本,返回结果。
pub fn run_script_with(script: &str, options: &QLOptions) -> Result<DataValue, QLException> {
    suite_runner()
        .execute(script, HashMap::new(), options)
        .map(|r| r.into_result())
}

/// 期望脚本执行成功(对应 suite 中无 errCode 的用例)。
pub fn expect_ok(script: &str) -> DataValue {
    run_script(script).unwrap_or_else(|err| panic!("脚本应执行成功但失败: {err:?}"))
}

/// 期望脚本执行成功且结果为 null(对应 suite 中 noReturn 用例)。
pub fn expect_null(script: &str) {
    let value = expect_ok(script);
    assert!(
        value.is_null(),
        "脚本应返回 null(noReturn),实际为 {value:?}"
    );
}

/// 期望脚本以指定错误码失败(对应 suite 中 errCode 用例,
/// Java `TestSuiteRunner.assertErrCode`)。
pub fn expect_err_code(script: &str, expected_code: &str) {
    match run_script(script) {
        Ok(value) => panic!("期望错误码 {expected_code},但脚本正常结束: {value:?}"),
        Err(err) => assert_eq!(
            err.error_code(),
            expected_code,
            "错误码不一致,实际错误: {err:?}"
        ),
    }
}

/// 指定选项 + 期望错误码。
pub fn expect_err_code_with(script: &str, options: &QLOptions, expected_code: &str) {
    match run_script_with(script, options) {
        Ok(value) => panic!("期望错误码 {expected_code},但脚本正常结束: {value:?}"),
        Err(err) => assert_eq!(
            err.error_code(),
            expected_code,
            "错误码不一致,实际错误: {err:?}"
        ),
    }
}

/// 指定选项 + 期望成功。
pub fn expect_ok_with(script: &str, options: &QLOptions) -> DataValue {
    run_script_with(script, options).unwrap_or_else(|err| panic!("脚本应执行成功但失败: {err:?}"))
}
