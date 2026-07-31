//! 对齐 Java `com.alibaba.qlexpress4.pf4j.Pf4jClassSupplierTest`。

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::class_supplier::ClassSupplier;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

struct PluginClassSupplier;

impl ClassSupplier for PluginClassSupplier {
    fn load_cls(&self, qualified_name: &str) -> Option<String> {
        (qualified_name == "com.alibaba.qlexpress4.pf4j.TestPluginInterface")
            .then(|| qualified_name.to_string())
    }
}

// Java source: Pf4jClassSupplierTest#testPluginClassSupplier
// ADAPTED: Rust 无 PF4J/JVM ClassLoader；宿主插件通过 ClassSupplier 暴露
// 类型名，并把可调用成员显式注册到 NativeRegistry，保留同一脚本和结果。
#[test]
fn explicit_plugin_supplier_exposes_registered_plugin_type() {
    let options = InitOptions::builder()
        .security_strategy(QLSecurityStrategy::open())
        .class_supplier(Rc::new(PluginClassSupplier))
        .build();
    let mut runner = Express4Runner::with_init_options(options);
    let mut plugin_type = NativeType::named("com.alibaba.qlexpress4.pf4j.TestPluginInterface");
    plugin_type.static_fields.insert(
        "TEST_CONSTANT".to_string(),
        DataValue::Str("Hello from PF4J Plugin!".into()),
    );
    runner.register_native_type(plugin_type);

    let result = runner
        .execute(
            concat!(
                "import com.alibaba.qlexpress4.pf4j.TestPluginInterface; ",
                "TestPluginInterface.TEST_CONSTANT"
            ),
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("plugin class")
        .into_result();
    assert_eq!(result, DataValue::Str("Hello from PF4J Plugin!".into()));
}
