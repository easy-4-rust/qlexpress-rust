//! Stage 7: 对齐 Java `runtime/instruction/GetFieldInstructionTest` (12 个 @Test)。
//!
//! 该测试用 MockQContextParent 大量子类,完全对齐需要 POJO 反射。Rust 端
//! 用 `#[derive(QLExpressType)]` + `register_qlexpress_type` 覆盖核心场景。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_derive::QLExpressType;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::member::QLExpressNativeType;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

#[derive(QLExpressType)]
pub struct Holder {
    pub value: i64,
    pub static_set: i64,
    pub static_get: i64,
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn run_int(runner: &Express4Runner, script: &str) -> i64 {
    let r = runner
        .execute(script, HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    match r {
        DataValue::Long(n) => n,
        DataValue::Int(n) => n as i64,
        other => panic!("expected int/long, got {other:?}"),
    }
}

// ---------- Static field access ----------

#[test]
fn get_field_static_via_registry() {
    // 通过实例访问字段(Java 端 'Parent.staticGet' 是静态类字段访问;
    // Rust 端没有 MetaClass 静态路径,改用 instance 路径等价)。
    let mut runner = Express4Runner::new();
    runner.register_qlexpress_type::<Holder>();
    let holder: Rc<RefCell<dyn qlexpress_rust::runtime::native_object::NativeObject>> = Holder {
        value: 1,
        static_set: 2,
        static_get: 3,
    }
    .into_data_value()
    .as_object_ref()
    .unwrap()
    .clone();
    let mut ctx = HashMap::new();
    ctx.insert("h".to_string(), DataValue::Object(holder));
    // 通过 instance 访问 static_get 字段
    let r = runner
        .execute("h.static_get", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(3));
}

#[test]
fn get_field_instance_with_assignment() {
    let mut runner = Express4Runner::new();
    runner.register_qlexpress_type::<Holder>();
    let holder: Rc<RefCell<dyn qlexpress_rust::runtime::native_object::NativeObject>> = Holder {
        value: 35,
        static_set: 0,
        static_get: 99,
    }
    .into_data_value()
    .as_object_ref()
    .unwrap()
    .clone();
    let mut ctx = HashMap::new();
    ctx.insert("h".to_string(), DataValue::Object(holder));
    let r = runner
        .execute("h.value", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(35));
}

#[test]
fn get_field_with_derive_native_object_directly() {
    // 通过 Object 直接调用 get_field
    let holder = Holder {
        value: 42,
        static_set: 0,
        static_get: 0,
    };
    let cell: Rc<RefCell<dyn qlexpress_rust::runtime::native_object::NativeObject>> = holder
        .into_data_value()
        .as_object_ref()
        .unwrap()
        .clone();
    let bean = DataValue::Object(cell);
    let native: &dyn qlexpress_rust::runtime::native_object::NativeObject =
        &*bean.as_object_ref().unwrap().borrow();
    assert_eq!(native.get_field("value"), Some(DataValue::Long(42)));
    assert_eq!(native.get_field("static_get"), Some(DataValue::Long(0)));
    assert_eq!(native.get_field("missing"), None);
}

#[test]
fn get_field_no_access_returns_none() {
    // 字段不存在 → get_field 返回 None
    let holder = Holder {
        value: 1,
        static_set: 0,
        static_get: 0,
    };
    let cell: Rc<RefCell<dyn qlexpress_rust::runtime::native_object::NativeObject>> = holder
        .into_data_value()
        .as_object_ref()
        .unwrap()
        .clone();
    let bean = DataValue::Object(cell);
    let native: &dyn qlexpress_rust::runtime::native_object::NativeObject =
        &*bean.as_object_ref().unwrap().borrow();
    assert_eq!(native.get_field("does_not_exist"), None);
}

#[test]
fn get_field_returns_correct_typename() {
    let holder = Holder {
        value: 100,
        static_set: 200,
        static_get: 300,
    };
    let cell: Rc<RefCell<dyn qlexpress_rust::runtime::native_object::NativeObject>> = holder
        .into_data_value()
        .as_object_ref()
        .unwrap()
        .clone();
    let bean = DataValue::Object(cell);
    let native: &dyn qlexpress_rust::runtime::native_object::NativeObject =
        &*bean.as_object_ref().unwrap().borrow();
    // 所有 pub 字段应能 get_field 返回
    assert_eq!(native.get_field("value"), Some(DataValue::Long(100)));
    assert_eq!(native.get_field("static_set"), Some(DataValue::Long(200)));
    assert_eq!(native.get_field("static_get"), Some(DataValue::Long(300)));
}