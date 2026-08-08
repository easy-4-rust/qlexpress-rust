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
use qlexpress::runtime::data::AssignableDataValue;
use qlexpress::runtime::delegate_qcontext::DelegateQContext;
use qlexpress::runtime::instruction::{
    MethodInvokeInstruction, NewInstanceInstruction, QLInstruction,
};
use qlexpress::runtime::left_value::LeftValue;
use qlexpress::runtime::member::NativeRegistry;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::{
    NativeConstructorCandidate, NativeMethodCandidate, NativeType,
};
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress::runtime::qvm_runtime::QvmRuntime;
use qlexpress::runtime::scope::QScope;
use qlexpress::runtime::value::{DataValue, QValue, Value};

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
    let registry = NativeRegistry::new();

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
        method(
            vec![ClassRef::array_of(named("java.lang.Object"))],
            false,
            |_, _| Ok(DataValue::Int(10)),
        ),
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
            vec![
                primitive(TargetType::Int),
                ClassRef::array_of(named("java.lang.String")),
            ],
            true,
            |_, _| Ok(DataValue::Str("1".into())),
        ),
    );
    child9.add_method_candidate(
        "addField1",
        method(
            vec![ClassRef::array_of(named("java.lang.Object"))],
            true,
            |_, _| Ok(DataValue::Str("1".into())),
        ),
    );
    child9.add_method_candidate(
        "addField2",
        method(
            vec![
                named("java.lang.Object"),
                ClassRef::array_of(named("java.lang.Object")),
            ],
            true,
            |_, _| Ok(DataValue::Str("1".into())),
        ),
    );
    child9.add_method_candidate(
        "addField3",
        method(
            vec![
                named("java.lang.Object"),
                ClassRef::array_of(ClassRef::Boxed(TargetType::Int)),
            ],
            true,
            |_, _| Ok(DataValue::Str("1".into())),
        ),
    );
    registry.register_type(child9);
    Rc::new(registry)
}

include!("alignment_member_overloads/methods.rs");
include!("alignment_member_overloads/constructors.rs");
