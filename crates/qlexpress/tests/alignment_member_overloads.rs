//! 逐项对齐 Java `MethodInvokeInstructionTest`（16 项）与
//! `NewInstanceInstructionTest`（10 项）。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::exception::error_codes;
use qlexpress::exception::error_reporter::ErrorReporter;
use qlexpress::exception::pure_err_reporter::PureErrReporter;
use qlexpress::exception::QLException;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::class_ref::ClassRef;
use qlexpress::runtime::data::convert::obj_type_convertor::TargetType;
use qlexpress::runtime::delegate_qcontext::DelegateQContext;
use qlexpress::runtime::instruction::{
    MethodInvokeInstruction, NewInstanceInstruction, QLInstruction,
};
use qlexpress::runtime::member::NativeRegistry;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::{
    NativeConstructorCandidate, NativeMethodCandidate, NativeType,
};
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress::runtime::qvm_runtime::QvmRuntime;
use qlexpress::runtime::scope::QScope;
use qlexpress::runtime::value::{DataValue, QValue};

fn reporter() -> Rc<dyn ErrorReporter> {
    Rc::new(PureErrReporter::INSTANCE)
}

fn named(name: &str) -> ClassRef {
    ClassRef::Named(name.to_string())
}

fn primitive(target: TargetType) -> ClassRef {
    ClassRef::Primitive(target)
}

fn method(
    parameter_types: Vec<ClassRef>,
    var_args: bool,
    body: impl Fn(&DataValue, &[DataValue]) -> Result<DataValue, QLException> + 'static,
) -> NativeMethodCandidate {
    NativeMethodCandidate::new(parameter_types, var_args, Rc::new(body))
}

fn constructor(
    parameter_types: Vec<ClassRef>,
    var_args: bool,
    body: impl Fn(&[DataValue]) -> Result<DataValue, QLException> + 'static,
) -> NativeConstructorCandidate {
    NativeConstructorCandidate::new(parameter_types, var_args, Rc::new(body))
}

struct FixtureObject {
    type_name: String,
    marker: DataValue,
}

impl FixtureObject {
    fn data(type_name: &str) -> DataValue {
        Self::with_marker(type_name, DataValue::Null)
    }

    fn with_marker(type_name: &str, marker: DataValue) -> DataValue {
        DataValue::Object(Rc::new(RefCell::new(Self {
            type_name: type_name.to_string(),
            marker,
        })))
    }
}

impl NativeObject for FixtureObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "marker").then(|| self.marker.clone())
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(QLException::for_test(
            qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
            format!("method not found: {name}"),
            error_codes::METHOD_NOT_FOUND,
        ))
    }

    fn native_type_name(&self) -> &str {
        &self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn context(registry: Rc<NativeRegistry>) -> DelegateQContext {
    let runtime = Rc::new(QvmRuntime::for_test(registry));
    let global = QScope::global(QvmGlobalScope::empty());
    let block = QScope::block_fresh_stack(&global, HashMap::new(), 16);
    DelegateQContext::new(runtime, block)
}

fn invoke(
    registry: Rc<NativeRegistry>,
    bean: DataValue,
    method_name: &str,
    args: Vec<DataValue>,
) -> Result<DataValue, QLException> {
    let mut context = context(registry);
    context.push(QValue::Data(bean));
    for arg in &args {
        context.push(QValue::Data(arg.clone()));
    }
    MethodInvokeInstruction::new(reporter(), method_name, args.len(), false)
        .execute(&mut context, &QLOptions::builder().build())?;
    Ok(context.pop().get())
}

fn instantiate(
    registry: Rc<NativeRegistry>,
    class_name: &str,
    args: Vec<DataValue>,
) -> Result<DataValue, QLException> {
    let mut context = context(registry);
    for arg in &args {
        context.push(QValue::Data(arg.clone()));
    }
    NewInstanceInstruction::new(reporter(), named(class_name), args.len())
        .execute(&mut context, &QLOptions::builder().build())?;
    Ok(context.pop().get())
}

fn method_registry() -> Rc<NativeRegistry> {
    let mut registry = NativeRegistry::new();

    let mut parent = NativeType::named("test.Parent");
    parent.add_method_candidate(
        "getMethod11",
        method(
            vec![primitive(TargetType::Int), primitive(TargetType::Int)],
            false,
            |_, args| {
                let [DataValue::Int(a), DataValue::Int(b)] = args else {
                    panic!("converted int arguments expected");
                };
                Ok(DataValue::Long(i64::from(a + b + 1)))
            },
        ),
    );
    parent.add_method_candidate(
        "getMethod12",
        method(
            vec![primitive(TargetType::Int), primitive(TargetType::Int)],
            false,
            |_, args| {
                let [DataValue::Int(a), DataValue::Int(b)] = args else {
                    panic!("converted int arguments expected");
                };
                Ok(DataValue::Long(i64::from(a + b)))
            },
        ),
    );
    registry.register_type(parent);

    let mut child = NativeType::named("test.Child");
    child.supertypes.push("test.Parent".to_string());
    child.add_method_candidate(
        "getMethod1",
        method(
            vec![primitive(TargetType::Int), primitive(TargetType::Int)],
            false,
            |_, args| {
                let [DataValue::Int(a), DataValue::Int(b)] = args else {
                    panic!("converted int arguments expected");
                };
                Ok(DataValue::Int(a + b))
            },
        ),
    );
    child.add_method_candidate(
        "getMethod11",
        method(
            vec![primitive(TargetType::Long), primitive(TargetType::Int)],
            false,
            |_, args| {
                let [DataValue::Long(a), DataValue::Int(b)] = args else {
                    panic!("long/int arguments expected");
                };
                Ok(DataValue::Long(a + i64::from(*b)))
            },
        ),
    );
    child.add_method_candidate(
        "getMethod12",
        method(
            vec![primitive(TargetType::Boolean), primitive(TargetType::Int)],
            false,
            |_, args| {
                let [DataValue::Bool(_), DataValue::Int(b)] = args else {
                    panic!("boolean/int arguments expected");
                };
                Ok(DataValue::Long(i64::from(*b)))
            },
        ),
    );
    registry.register_type(child);

    let mut runnable = NativeType::named("java.lang.Runnable");
    runnable.add_method_candidate("run", method(Vec::new(), false, |_, _| Ok(DataValue::Null)));
    registry.register_type(runnable);

    let mut default_method = NativeType::named("test.InterWithDefaultMethod");
    default_method.add_method_candidate(
        "returnI",
        method(Vec::new(), false, |_, _| Ok(DataValue::Int(9))),
    );
    registry.register_type(default_method);

    let mut child3 = NativeType::named("test.Child3");
    child3.supertypes.push("test.Parent".to_string());
    child3.add_method_candidate(
        "getMethod5",
        method(vec![named("test.Parent")], false, |_, _| {
            Ok(DataValue::Int(0))
        }),
    );
    child3.add_method_candidate(
        "getMethod6",
        method(vec![named("java.lang.Object[]")], false, |_, _| {
            Ok(DataValue::Int(10))
        }),
    );
    registry.register_type(child3);

    let mut child4 = NativeType::named("test.Child4");
    child4.add_method_candidate(
        "getMethod7",
        method(vec![primitive(TargetType::Int)], false, |_, args| {
            Ok(args[0].clone())
        }),
    );
    registry.register_type(child4);

    let mut child5 = NativeType::named("test.Child5");
    child5.add_method_candidate(
        "getMethod8",
        method(vec![primitive(TargetType::Double)], false, |_, args| {
            Ok(args[0].clone())
        }),
    );
    registry.register_type(child5);

    let mut child6 = NativeType::named("test.Child6");
    child6.add_method_candidate(
        "getMethod9",
        method(
            vec![primitive(TargetType::BigInteger)],
            false,
            |_, args| match &args[0] {
                DataValue::BigInt(value) => Ok(DataValue::Int(value.to_string().parse().unwrap())),
                _ => panic!("BigInteger argument expected"),
            },
        ),
    );
    child6.add_method_candidate(
        "getMethod10",
        method(
            vec![primitive(TargetType::Double)],
            false,
            |_, args| match args[0] {
                DataValue::Double(value) => Ok(DataValue::BigDec(value.to_string())),
                _ => panic!("double argument expected"),
            },
        ),
    );
    registry.register_type(child6);

    let mut child9 = NativeType::named("test.Child9");
    child9.add_method_candidate(
        "addField",
        method(
            vec![primitive(TargetType::Int), named("java.lang.String")],
            true,
            |_, _| Ok(DataValue::Str("1".to_string())),
        ),
    );
    child9.add_method_candidate(
        "addField1",
        method(vec![named("java.lang.Object")], true, |_, _| {
            Ok(DataValue::Str("1".to_string()))
        }),
    );
    child9.add_method_candidate(
        "addField2",
        method(
            vec![named("java.lang.Object"), named("java.lang.Object")],
            true,
            |_, _| Ok(DataValue::Str("1".to_string())),
        ),
    );
    child9.add_method_candidate(
        "addField3",
        method(
            vec![named("java.lang.Object"), primitive(TargetType::Int)],
            true,
            |_, _| Ok(DataValue::Str("1".to_string())),
        ),
    );
    registry.register_type(child9);
    Rc::new(registry)
}

#[test]
/// Java `MethodInvokeInstructionTest#equalTypeTest`。
fn java_method_equal_type_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child"),
            "getMethod1",
            vec![DataValue::Int(1), DataValue::Int(2)],
        )
        .unwrap(),
        DataValue::Int(3)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#runnableTest`。
fn java_method_runnable_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("java.lang.Runnable"),
            "run",
            vec![],
        )
        .unwrap(),
        DataValue::Null
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#defaultMethodTest`。
fn java_method_default_method_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.InterWithDefaultMethod"),
            "returnI",
            vec![],
        )
        .unwrap(),
        DataValue::Int(9)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#childMethodMatchTest`。
fn java_method_child_method_match_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child"),
            "getMethod11",
            vec![DataValue::Int(1), DataValue::Int(2)],
        )
        .unwrap(),
        DataValue::Long(3)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#parentMethodMatch`。
fn java_method_parent_method_match_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child"),
            "getMethod12",
            vec![DataValue::Int(1), DataValue::Int(2)],
        )
        .unwrap(),
        DataValue::Long(3)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#convertTypeAssignedMatch`。
fn java_method_convert_type_assigned_match_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child3"),
            "getMethod5",
            vec![FixtureObject::data("test.Child3")],
        )
        .unwrap(),
        DataValue::Int(0)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#arrayParamTest`。
fn java_method_array_param_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child3"),
            "getMethod6",
            vec![DataValue::array(vec![DataValue::Int(5), DataValue::Int(6)])],
        )
        .unwrap(),
        DataValue::Int(10)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#primitiveParamTest`。
fn java_method_primitive_param_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child4"),
            "getMethod7",
            vec![DataValue::Int(5)],
        )
        .unwrap(),
        DataValue::Int(5)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#primitiveImplicitTest`。
fn java_method_primitive_implicit_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child5"),
            "getMethod8",
            vec![DataValue::Int(5)],
        )
        .unwrap(),
        DataValue::Double(5.0)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#bigIntegerImplicitTest`。
fn java_method_big_integer_implicit_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child6"),
            "getMethod9",
            vec![DataValue::Int(5)],
        )
        .unwrap(),
        DataValue::Int(5)
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#doubleMatchTest`。
fn java_method_double_match_test() {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child6"),
            "getMethod10",
            vec![DataValue::Float(5.0)],
        )
        .unwrap(),
        DataValue::BigDec("5".to_string())
    );
}

fn assert_child9(method_name: &str) {
    assert_eq!(
        invoke(
            method_registry(),
            FixtureObject::data("test.Child9"),
            method_name,
            vec![
                DataValue::Int(5),
                DataValue::Str("5.0".to_string()),
                DataValue::Str("5.0".to_string()),
            ],
        )
        .unwrap(),
        DataValue::Str("1".to_string())
    );
}

#[test]
/// Java `MethodInvokeInstructionTest#varArgTest`。
fn java_method_var_arg_test() {
    assert_child9("addField");
}

#[test]
/// Java `MethodInvokeInstructionTest#varArgTest2`。
fn java_method_var_arg_test2() {
    assert_child9("addField1");
}

#[test]
/// Java `MethodInvokeInstructionTest#varArgTest3`。
fn java_method_var_arg_test3() {
    assert_child9("addField2");
}

#[test]
/// Java `MethodInvokeInstructionTest#varArgNotMatchTest`。
fn java_method_var_arg_not_match_test() {
    let error = invoke(
        method_registry(),
        FixtureObject::data("test.Child9"),
        "addField3",
        vec![
            DataValue::Int(5),
            DataValue::Str("asd".to_string()),
            DataValue::Str("sss".to_string()),
        ],
    )
    .unwrap_err();
    assert_eq!(error.error_code(), error_codes::METHOD_NOT_FOUND);
}

#[test]
/// Java `MethodInvokeInstructionTest#varArgNotMatchTest2`。
fn java_method_var_arg_not_match_test2() {
    let error = invoke(
        method_registry(),
        FixtureObject::data("test.Child9"),
        "addField",
        vec![DataValue::Int(5), DataValue::Int(1), DataValue::Int(1)],
    )
    .unwrap_err();
    assert_eq!(error.error_code(), error_codes::METHOD_NOT_FOUND);
}

fn constructor_registry() -> Rc<NativeRegistry> {
    let mut registry = NativeRegistry::new();

    let mut parent = NativeType::named("test.Parent");
    parent.add_constructor_candidate(constructor(Vec::new(), false, |_| {
        Ok(FixtureObject::data("test.Parent"))
    }));
    parent.add_constructor_candidate(constructor(
        vec![primitive(TargetType::Int)],
        false,
        |args| Ok(FixtureObject::with_marker("test.Parent", args[0].clone())),
    ));
    registry.register_type(parent);

    let mut child = NativeType::named("test.Child");
    child.add_constructor_candidate(constructor(Vec::new(), false, |_| {
        Ok(FixtureObject::data("test.Child"))
    }));
    registry.register_type(child);

    let mut child1 = NativeType::named("test.Child1");
    child1.add_constructor_candidate(constructor(
        vec![primitive(TargetType::Long), primitive(TargetType::Int)],
        false,
        |_| Ok(FixtureObject::data("test.Child1")),
    ));
    registry.register_type(child1);

    let mut child3 = NativeType::named("test.Child3");
    child3.supertypes.push("test.Parent".to_string());
    child3.add_constructor_candidate(constructor(vec![named("test.Parent")], false, |_| {
        Ok(FixtureObject::with_marker("test.Child3", DataValue::Int(1)))
    }));
    child3.add_constructor_candidate(constructor(
        vec![named("java.lang.Object[]")],
        false,
        |_| Ok(FixtureObject::with_marker("test.Child3", DataValue::Int(2))),
    ));
    registry.register_type(child3);

    let mut child4 = NativeType::named("test.Child4");
    child4.add_constructor_candidate(constructor(vec![primitive(TargetType::Int)], false, |_| {
        Ok(FixtureObject::data("test.Child4"))
    }));
    registry.register_type(child4);

    let mut child5 = NativeType::named("test.Child5");
    child5.add_constructor_candidate(constructor(
        vec![primitive(TargetType::Double)],
        false,
        |_| Ok(FixtureObject::data("test.Child5")),
    ));
    registry.register_type(child5);

    let mut child6 = NativeType::named("test.Child6");
    child6.add_constructor_candidate(constructor(
        vec![primitive(TargetType::Double)],
        false,
        |_| Ok(FixtureObject::with_marker("test.Child6", DataValue::Int(1))),
    ));
    child6.add_constructor_candidate(constructor(
        vec![primitive(TargetType::BigInteger)],
        false,
        |_| Ok(FixtureObject::with_marker("test.Child6", DataValue::Int(2))),
    ));
    registry.register_type(child6);

    let mut number = NativeType::named("test.NumberConstructor");
    number.add_constructor_candidate(constructor(
        vec![primitive(TargetType::Double)],
        false,
        |_| {
            Ok(FixtureObject::with_marker(
                "test.NumberConstructor",
                DataValue::Int(0),
            ))
        },
    ));
    number.add_constructor_candidate(constructor(vec![named("java.lang.Number")], false, |_| {
        Ok(FixtureObject::with_marker(
            "test.NumberConstructor",
            DataValue::Int(1),
        ))
    }));
    number.add_constructor_candidate(constructor(
        vec![primitive(TargetType::BigDecimal)],
        false,
        |_| {
            Ok(FixtureObject::with_marker(
                "test.NumberConstructor",
                DataValue::Int(2),
            ))
        },
    ));
    number.add_constructor_candidate(constructor(vec![named("java.lang.String")], false, |_| {
        Ok(FixtureObject::with_marker(
            "test.NumberConstructor",
            DataValue::Int(3),
        ))
    }));
    registry.register_type(number);

    let mut child9 = NativeType::named("test.Child9");
    child9.add_constructor_candidate(constructor(
        vec![primitive(TargetType::Int), named("java.lang.String")],
        true,
        |_| Ok(FixtureObject::data("test.Child9")),
    ));
    registry.register_type(child9);
    Rc::new(registry)
}

fn assert_object_type(value: &DataValue, expected: &str) {
    let DataValue::Object(object) = value else {
        panic!("native object expected");
    };
    assert_eq!(object.borrow().native_type_name(), expected);
}

fn marker(value: &DataValue) -> DataValue {
    let DataValue::Object(object) = value else {
        panic!("native object expected");
    };
    object.borrow().get_field("marker").unwrap()
}

#[test]
/// Java `NewInstanceInstructionTest#newInstructionTest`。
fn java_constructor_new_instruction_test() {
    let registry = constructor_registry();
    let zero = instantiate(Rc::clone(&registry), "test.Parent", vec![]).unwrap();
    assert_object_type(&zero, "test.Parent");
    let with_age = instantiate(registry, "test.Parent", vec![DataValue::Int(6)]).unwrap();
    assert_object_type(&with_age, "test.Parent");
    assert_eq!(marker(&with_age), DataValue::Int(6));
}

#[test]
/// Java `NewInstanceInstructionTest#constructorNotFoundTest`。
fn java_constructor_not_found_test() {
    let error = instantiate(
        constructor_registry(),
        "test.Child",
        vec![DataValue::Int(2), DataValue::Int(3)],
    )
    .unwrap_err();
    assert_eq!(error.error_code(), error_codes::NO_SUITABLE_CONSTRUCTOR);
}

#[test]
/// Java `NewInstanceInstructionTest#constructorConvertMatchTest`。
fn java_constructor_convert_match_test() {
    let value = instantiate(
        constructor_registry(),
        "test.Child1",
        vec![DataValue::Int(2), DataValue::Int(3)],
    )
    .unwrap();
    assert_object_type(&value, "test.Child1");
}

#[test]
/// Java `NewInstanceInstructionTest#constructorConvertAssignedMatch`。
fn java_constructor_convert_assigned_match_test() {
    let value = instantiate(
        constructor_registry(),
        "test.Child3",
        vec![FixtureObject::data("test.Child3")],
    )
    .unwrap();
    assert_object_type(&value, "test.Child3");
    assert_eq!(marker(&value), DataValue::Int(1));
}

#[test]
/// Java `NewInstanceInstructionTest#arrayParamTest`。
fn java_constructor_array_param_test() {
    let value = instantiate(
        constructor_registry(),
        "test.Child3",
        vec![DataValue::array(vec![DataValue::Int(5), DataValue::Int(6)])],
    )
    .unwrap();
    assert_object_type(&value, "test.Child3");
    assert_eq!(marker(&value), DataValue::Int(2));
}

#[test]
/// Java `NewInstanceInstructionTest#primitiveParamTest`。
fn java_constructor_primitive_param_test() {
    let value = instantiate(
        constructor_registry(),
        "test.Child4",
        vec![DataValue::Int(5)],
    )
    .unwrap();
    assert_object_type(&value, "test.Child4");
}

#[test]
/// Java `NewInstanceInstructionTest#primitiveImplicitTest`。
fn java_constructor_primitive_implicit_test() {
    let value = instantiate(
        constructor_registry(),
        "test.Child5",
        vec![DataValue::Int(5)],
    )
    .unwrap();
    assert_object_type(&value, "test.Child5");
}

#[test]
/// Java `NewInstanceInstructionTest#bigIntegerImplicitTest`。
fn java_constructor_big_integer_implicit_test() {
    let value = instantiate(
        constructor_registry(),
        "test.Child6",
        vec![DataValue::Int(5)],
    )
    .unwrap();
    assert_object_type(&value, "test.Child6");
    assert_eq!(marker(&value), DataValue::Int(2));
}

#[test]
/// Java `NewInstanceInstructionTest#numberConstructorMatchTest`。
fn java_constructor_number_match_test() {
    let value = instantiate(
        constructor_registry(),
        "test.NumberConstructor",
        vec![DataValue::Double(5.0)],
    )
    .unwrap();
    assert_eq!(marker(&value), DataValue::Int(0));
}

#[test]
/// Java `NewInstanceInstructionTest#varArgTest`。
fn java_constructor_var_arg_test() {
    let value = instantiate(
        constructor_registry(),
        "test.Child9",
        vec![
            DataValue::Int(5),
            DataValue::Str("5.0".to_string()),
            DataValue::Str("5.0".to_string()),
        ],
    )
    .unwrap();
    assert_object_type(&value, "test.Child9");
}
