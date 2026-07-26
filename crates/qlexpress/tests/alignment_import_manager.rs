//! Stage 7: 对齐 Java `aparser/ImportManagerTest` (2 个 @Test)。
//!
//! 核心契约:default import 列表用于解析 unqualified class name。
//! Rust 端通过 `InitOptions::add_default_import` 实现。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_rust::aparser::import_manager::QLImport;
use qlexpress_rust::default_class_supplier::DefaultClassSupplier;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress_rust::Express4Runner;

fn runner_with_default_import(imports: Vec<QLImport>) -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(imports)
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    )
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

#[test]
fn default_import_unqualified_class() {
    // 模拟 Java loadTest: 添加 default import 后, unqualified class name
    // 可以解析。这里使用 com.example.Calc + 注册 Calc 类型。
    let mut runner = runner_with_default_import(vec![QLImport::import_cls("com.example.Calc")]);

    // 注册一个 static method 在 Calc 上
    use qlexpress_rust::runtime::native_type::NativeType;
    let mut calc = NativeType::named("com.example.Calc");
    calc.static_methods.insert(
        "answer".to_string(),
        std::rc::Rc::new(|_bean, _args| Ok(qlexpress_rust::runtime::value::DataValue::Long(42))),
    );
    runner.register_native_type(calc);

    // 用 unqualified 'Calc' 访问 default-imported 类型
    let r = runner
        .execute("Calc.answer()", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, qlexpress_rust::runtime::value::DataValue::Long(42));
}

#[test]
fn without_default_import_unqualified_fails() {
    // 没有 default import,unqualified class 不可解析
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let r = runner.execute("Calc.answer()", HashMap::new(), &opts());
    assert!(r.is_err());
}