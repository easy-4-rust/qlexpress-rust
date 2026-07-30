//! 对齐 Java `com.alibaba.qlexpress4.spring.SpringDemoTest`。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::exception::error_codes;
use qlexpress::exception::ql_exception::QLExceptionKind;
use qlexpress::exception::QLException;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

struct HelloService;

impl NativeObject for HelloService {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("hello", [DataValue::Str(name)]) => Ok(DataValue::Str(format!("Hello, {name}!"))),
            _ => Err(QLException::for_test(
                QLExceptionKind::Runtime,
                format!("method not found: {name}"),
                error_codes::METHOD_NOT_FOUND,
            )),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.spring.HelloService"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Java source: SpringDemoTest#qlExecuteWithSpringContextTest
// ADAPTED: Spring @Autowired 容器注入在 Rust 宿主中改为显式对象注入；
// 传入表达式的 bean、上下文变量、脚本和结果均与 Java 测试一致。
#[test]
fn ql_execute_with_explicit_host_context() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let context = HashMap::from([
        (
            "helloService".to_string(),
            DataValue::Object(Rc::new(RefCell::new(HelloService))),
        ),
        ("name".to_string(), DataValue::Str("Wang".to_string())),
    ]);

    let result = runner
        .execute("helloService.hello(name)", context, &QLOptions::default())
        .expect("host service")
        .into_result();
    assert_eq!(result, DataValue::Str("Hello, Wang!".to_string()));
}
