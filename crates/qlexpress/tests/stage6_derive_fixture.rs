//! Stage 6 fixture tests for `#[derive(QLExpressType)]`.
//!
//! Each fixture struct exercises one or more code-generation paths of
//! the proc-macro. The point of the suite is to lock down the contract
//! between the macro and the runtime: any future change to either side
//! must keep these tests green.
//!
//! Test coverage:
//! - field getters (scalars)
//! - `no_native_object` opt-out
//! - `#[qlexpress(skip)]` on a field
//! - field aliases via `#[qlexpress(alias = "X")]` (registry path)
//! - custom canonical name via `#[qlexpress(name = "...")]`
//! - `register_qlexpress_type::<T>()` convenience method on the runner

#![allow(clippy::result_large_err)]

use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions as QlOpts;
use qlexpress::runtime::member::{QLExpressNativeType, QLExpressRegistryExt};
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_registry::NativeRegistry;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::{Express4Runner, QLExpressType};
use std::rc::Rc;

// ---------------- Fixture 1: simple scalars ----------------

#[derive(QLExpressType)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

#[test]
fn derive_simple_struct_builds_native_type() {
    let nt = Point::build_native_type();
    assert_eq!(nt.name, "Point");
    assert!(nt.fields.contains_key("x"));
    assert!(nt.fields.contains_key("y"));
    assert_eq!(nt.fields.len(), 2);
}

#[test]
fn derive_field_getter_returns_correct_value() {
    let nt = Point::build_native_type();
    let bean: DataValue = Point { x: 7, y: 11 }.into_data_value();
    let getter = nt.fields.get("x").unwrap();
    assert_eq!(getter(&bean), Some(DataValue::Long(7)));
    let getter = nt.fields.get("y").unwrap();
    assert_eq!(getter(&bean), Some(DataValue::Long(11)));
}

#[test]
fn derive_native_object_get_field() {
    let bean: DataValue = Point { x: 42, y: 99 }.into_data_value();
    let obj = bean.as_object_ref().unwrap();
    let native: &dyn NativeObject = &*obj.borrow();
    assert_eq!(native.get_field("x"), Some(DataValue::Long(42)));
    assert_eq!(native.get_field("y"), Some(DataValue::Long(99)));
    assert_eq!(native.get_field("missing"), None);
    assert_eq!(native.native_type_name(), "Point");
}

#[test]
fn derive_native_object_writes_supported_fields() {
    let bean: DataValue = Point { x: 1, y: 2 }.into_data_value();
    let object = bean.as_object_ref().expect("derived object");
    assert!(object.borrow_mut().set_field("x", &DataValue::Int(42)));
    assert_eq!(object.borrow().get_field("x"), Some(DataValue::Long(42)));
    assert!(!object
        .borrow_mut()
        .set_field("x", &DataValue::Str("invalid".to_string())));
    assert!(!object.borrow_mut().set_field("missing", &DataValue::Int(1)));
}

// ---------------- Fixture 2: name override + skip + alias ----------------

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.Rect")]
pub struct Rect {
    pub w: i64,
    #[qlexpress(skip)]
    pub internal_id: i64,
    #[qlexpress(readonly)]
    pub immutable: i64,
    #[qlexpress(alias("height", "h"))]
    pub h: i64,
}

#[test]
fn derive_uses_custom_name() {
    let nt = Rect::build_native_type();
    assert_eq!(nt.name, "com.example.Rect");
}

#[test]
fn derive_skips_marked_field() {
    let nt = Rect::build_native_type();
    assert!(!nt.fields.contains_key("internal_id"));
    assert!(nt.fields.contains_key("w"));
    assert!(nt.fields.contains_key("immutable"));
    assert!(nt.fields.contains_key("h"));
}

#[test]
fn derive_records_field_aliases() {
    let nt = Rect::build_native_type();
    let aliases = nt.field_aliases.get("h").expect("aliases for h");
    assert!(aliases.contains(&"height".to_string()));
    assert!(aliases.contains(&"h".to_string()));
}

#[test]
fn derive_native_object_resolves_alias() {
    let bean: DataValue = Rect {
        w: 5,
        internal_id: 999,
        immutable: 7,
        h: 8,
    }
    .into_data_value();
    let obj = bean.as_object_ref().unwrap();
    {
        let borrowed = obj.borrow();
        let native: &dyn NativeObject = &*borrowed;
        assert_eq!(native.get_field("h"), Some(DataValue::Long(8)));
        assert_eq!(native.get_field("height"), Some(DataValue::Long(8)));
        assert_eq!(native.get_field("w"), Some(DataValue::Long(5)));
        // skipped fields are not visible via get_field
        assert_eq!(native.get_field("internal_id"), None);
    }
    assert!(!bean
        .as_object_ref()
        .expect("rect object")
        .borrow_mut()
        .set_field("immutable", &DataValue::Long(9)));
}

// ---------------- Fixture 7: Java classified object literal ----------------

#[derive(Default, QLExpressType)]
#[qlexpress(name = "com.example.ClassifiedRecord")]
struct ClassifiedRecord {
    name: String,
    score: i64,
}

#[test]
fn classified_object_literal_constructs_and_populates_native_object() {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.ClassifiedRecord");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let mut native_type = ClassifiedRecord::build_native_type();
    native_type.constructor = Some(Rc::new(|args| {
        assert!(
            args.is_empty(),
            "classified object uses the no-arg constructor"
        );
        Ok(ClassifiedRecord::default().into_data_value())
    }));
    runner.register_native_type(native_type);

    let result = runner
        .execute(
            "{'@class':'com.example.ClassifiedRecord', 'name':'alice', 'score':7}",
            std::collections::HashMap::new(),
            &opts(),
        )
        .expect("classified object literal executes")
        .into_result();
    let object = result.as_object_ref().expect("native object result");
    assert_eq!(
        object.borrow().get_field("name"),
        Some(DataValue::Str("alice".to_string()))
    );
    assert_eq!(object.borrow().get_field("score"), Some(DataValue::Long(7)));
}

// ---------------- Fixture 3: registry path ----------------

#[derive(QLExpressType)]
pub struct Box2D {
    pub left: i64,
    pub top: i64,
}

#[test]
fn registry_ext_trait_registers_derived_type() {
    let mut reg = NativeRegistry::with_builtins();
    reg.register_qlexpress_type::<Box2D>();
    assert!(reg.get_type("Box2D").is_some());
}

#[test]
fn express4_runner_register_qlexpress_type() {
    let mut runner = Express4Runner::new();
    // RUST_OBLIGATION: 已创建的 QVM/Lambda 会持有同一个注册表句柄；
    // 后续注册必须对既有句柄可见，不能因 `Rc` 已共享而 panic 或复制分叉。
    let registry = runner.registry().clone();
    runner.register_qlexpress_type::<Box2D>();
    assert!(registry.get_type("Box2D").is_some());
}

#[test]
fn runner_can_resolve_derived_field_via_registry() {
    let mut runner = Express4Runner::new();
    runner.register_qlexpress_type::<Box2D>();
    let bean: DataValue = Box2D { left: 1, top: 2 }.into_data_value();
    let getter = runner
        .registry()
        .get_type("Box2D")
        .unwrap()
        .fields
        .get("top")
        .unwrap()
        .clone();
    assert_eq!(getter(&bean), Some(DataValue::Long(2)));
}

// ---------------- Fixture 4: numeric + string + bool fields ----------------

#[derive(QLExpressType)]
pub struct Stats {
    pub n: i64,
    pub f: f64,
    pub b: bool,
    pub s: String,
}

#[test]
fn derive_converts_mixed_field_values() {
    let s = Stats {
        n: 5,
        f: 1.5,
        b: true,
        s: "hi".to_string(),
    };
    let bean: DataValue = s.into_data_value();
    let obj = bean.as_object_ref().unwrap();
    let native: &dyn NativeObject = &*obj.borrow();
    assert_eq!(native.get_field("n"), Some(DataValue::Long(5)));
    assert_eq!(native.get_field("f"), Some(DataValue::Double(1.5)));
    assert_eq!(native.get_field("b"), Some(DataValue::Bool(true)));
    assert_eq!(
        native.get_field("s"),
        Some(DataValue::Str("hi".to_string()))
    );
}

// ---------------- Fixture 6: round-trip execute via runner ----------------

fn opts() -> QlOpts {
    QlOpts::builder().build()
}

#[test]
fn script_can_read_field_of_derived_type() {
    let mut runner = Express4Runner::with_init_options(
        qlexpress::init_options::InitOptions::builder()
            .security_strategy(
                qlexpress::security::ql_security_strategy::QLSecurityStrategy::open(),
            )
            .build(),
    );
    runner.register_qlexpress_type::<Box2D>();
    let bean: DataValue = Box2D { left: 10, top: 20 }.into_data_value();
    let mut ctx = std::collections::HashMap::new();
    ctx.insert("b".to_string(), bean);
    let result = runner
        .execute("b.left + b.top", ctx, &opts())
        .expect("exec ok");
    assert_eq!(result.into_result(), DataValue::Long(30));
}
