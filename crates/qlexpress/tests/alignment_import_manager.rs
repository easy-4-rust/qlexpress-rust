//! Stage 7: 对齐 Java `aparser/ImportManagerTest` (2 个 @Test)。
//!
//! 核心契约:default import 列表用于解析 unqualified class name。
//! Rust 端通过 `InitOptions::add_default_import` 实现。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

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
    use qlexpress::runtime::native_type::NativeType;
    let mut calc = NativeType::named("com.example.Calc");
    calc.static_methods.insert(
        "answer".to_string(),
        std::rc::Rc::new(|_bean, _args| Ok(qlexpress::runtime::value::DataValue::Long(42))),
    );
    runner.register_native_type(calc);

    // 用 unqualified 'Calc' 访问 default-imported 类型
    let r = runner
        .execute("Calc.answer()", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, qlexpress::runtime::value::DataValue::Long(42));
}

#[test]
fn without_default_import_unqualified_fails() {
    // 没有 default import,unqualified class 不可解析
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let r = runner.execute("Calc.answer()", HashMap::new(), &opts());
    assert!(r.is_err());
}

/// SOURCE_PARITY: Java `InitOptions#getDefaultImport()` 返回实际可变 List；
/// runner 创建后追加默认导入，后续编译必须立即读取到新条目。
#[test]
fn existing_runner_observes_later_default_import_mutation() {
    use qlexpress::runtime::native_type::NativeType;
    use qlexpress::runtime::value::DataValue;

    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    let options = InitOptions::builder()
        .class_supplier(Rc::new(supplier))
        .security_strategy(QLSecurityStrategy::open())
        .build();
    let retained_options = options.clone();
    let mut runner = Express4Runner::with_init_options(options);
    let mut calc = NativeType::named("com.example.Calc");
    calc.static_methods.insert(
        "answer".to_string(),
        Rc::new(|_bean, _args| Ok(DataValue::Long(42))),
    );
    runner.register_native_type(calc);

    assert!(
        runner
            .execute("Calc.answer()", HashMap::new(), &opts())
            .is_err(),
        "unqualified type must not resolve before the late import"
    );

    retained_options
        .default_import_mut()
        .push(QLImport::import_cls("com.example.Calc"));
    assert_eq!(
        runner
            .execute("Calc.answer()", HashMap::new(), &opts())
            .expect("existing runner must observe the late import")
            .into_result(),
        DataValue::Long(42)
    );
}
