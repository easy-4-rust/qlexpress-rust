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
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::NativeType;
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
    ("com.alibaba.qlexpress4.inport", "Person"),
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
