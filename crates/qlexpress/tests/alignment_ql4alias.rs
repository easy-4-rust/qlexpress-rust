//! Stage 7: 对齐 Java `test/annotation/QL4AliasTest` (3 个 @Test)。
//!
//! QL4Alias 是 Java 注解,支持 Chinese / unicode 字段名/方法名别名。
//! Rust 端用 #[qlexpress(alias(...))] derive 属性 + register 时设置
//! NativeType.name 模拟。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_derive::QLExpressType;
use qlexpress_rust::aparser::import_manager::QLImport;
use qlexpress_rust::default_class_supplier::DefaultClassSupplier;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::member::QLExpressNativeType;
use qlexpress_rust::runtime::native_type::NativeType;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress_rust::Express4Runner;

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.Child7")]
pub struct Child7 {
    #[qlexpress(alias("测试静态字段"))]
    pub value: i64,
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn runner() -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Child7");
    Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(vec![QLImport::import_cls("com.example.Child7")])
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    )
}

#[test]
fn alias_field_access_chinese() {
    // 验证 Rust 端 #[qlexpress(alias)] 能产生 alias 列表 (Chinese name)
    let nt = Child7::build_native_type();
    let aliases = nt.field_aliases.get("value");
    assert!(aliases.is_some());
    assert!(aliases.unwrap().contains(&"测试静态字段".to_string()));
}

#[test]
fn alias_field_access_via_runner() {
    let mut runner = runner();
    runner.register_qlexpress_type::<Child7>();
    let child: DataValue = Child7 { value: 8 }.into_data_value();
    let mut ctx = HashMap::new();
    ctx.insert("c".to_string(), child);
    // 通过 alias "测试静态字段" 访问
    let r = runner
        .execute("c.测试静态字段", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(8));
}

#[test]
fn default_field_name_works() {
    // 默认 field name "value" 也应能访问
    let mut runner = runner();
    runner.register_qlexpress_type::<Child7>();
    let child: DataValue = Child7 { value: 42 }.into_data_value();
    let mut ctx = HashMap::new();
    ctx.insert("c".to_string(), child);
    let r = runner
        .execute("c.value", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(42));
}

// Silence unused NativeType warning from in-test reference.
#[allow(dead_code)]
fn _type_ref() -> NativeType {
    Child7::build_native_type()
}
