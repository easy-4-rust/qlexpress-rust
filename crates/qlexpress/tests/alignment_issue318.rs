//! Stage 7: 对齐 Java `test/issue/Issue318Test` — 直接访问 public field
//! 不需要 getter。Rust 端的 `#[derive(QLExpressType)]` 通过 `NativeObject`
//! 实现处理字段读取,脚本中 `obj.field` 即可读取。

#![allow(clippy::result_large_err)]

mod alignment_util;

use std::collections::HashMap;

use qlexpress_derive::QLExpressType;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::member::QLExpressNativeType;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

#[derive(QLExpressType)]
pub struct Student {
    pub name: String,
    pub alias: String,
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

#[test]
fn direct_field_access_no_getter_needed() {
    let mut runner = Express4Runner::new();
    runner.register_qlexpress_type::<Student>();

    let student = Student {
        name: "zhangsan".to_string(),
        alias: "zs".to_string(),
    };
    let bean: DataValue = student.into_data_value();
    let mut ctx = HashMap::new();
    ctx.insert("s".to_string(), bean);

    let r = runner
        .execute("s.name", ctx.clone(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("zhangsan".to_string()));

    let r2 = runner
        .execute("s.alias", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r2, DataValue::Str("zs".to_string()));
}

#[test]
fn direct_field_access_in_boolean_expression() {
    let mut runner = Express4Runner::new();
    runner.register_qlexpress_type::<Student>();

    let student = Student {
        name: "zhangsan".to_string(),
        alias: "zs".to_string(),
    };
    let bean: DataValue = student.into_data_value();
    let mut ctx = HashMap::new();
    ctx.insert("s".to_string(), bean);

    let r = runner
        .execute("s.name == \"zhangsan\"", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Bool(true));
}

// Silence alignment_util warning when only used by other suites.
#[allow(dead_code)]
fn _unused_helper() {
    let _ = alignment_util::expect_ok("1 + 1");
}
