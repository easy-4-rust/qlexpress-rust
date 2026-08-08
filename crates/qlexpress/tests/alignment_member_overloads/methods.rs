/// SOURCE_PARITY: Java `ObjTypeConvertor#noNeedConvert` 使用
/// `Class#isInstance`，因此声明为父类的变量接受注册子类实例，但拒绝无关类型。
#[test]
fn named_declared_type_preserves_registered_assignability() {
    let registry = method_registry();
    let mut slot = AssignableDataValue::with_class(
        "value",
        DataValue::Null,
        named("test.Parent"),
        Rc::clone(&registry),
    );

    slot.set(
        FixtureObject::data("test.Child"),
        &PureErrReporter::INSTANCE,
    )
    .expect("registered child must be assignable to parent");
    let DataValue::Object(current) = slot.get() else {
        panic!("host object expected");
    };
    assert_eq!(current.borrow().native_type_name(), "test.Child");

    let error = slot
        .set(
            DataValue::Str("not a parent".into()),
            &PureErrReporter::INSTANCE,
        )
        .unwrap_err();
    assert_eq!(
        error.error_code(),
        error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE
    );
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
                DataValue::Str("5.0".into()),
                DataValue::Str("5.0".into()),
            ],
        )
        .unwrap(),
        DataValue::Str("1".into())
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
            DataValue::Str("asd".into()),
            DataValue::Str("sss".into()),
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
