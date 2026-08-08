fn constructor_registry() -> Rc<NativeRegistry> {
    let registry = NativeRegistry::new();

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
        vec![ClassRef::array_of(named("java.lang.Object"))],
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
        vec![
            primitive(TargetType::Int),
            ClassRef::array_of(named("java.lang.String")),
        ],
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
            DataValue::Str("5.0".into()),
            DataValue::Str("5.0".into()),
        ],
    )
    .unwrap();
    assert_object_type(&value, "test.Child9");
}
