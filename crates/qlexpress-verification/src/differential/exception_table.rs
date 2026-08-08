fn execute_exception_table(
    id: String,
    invocation: ExceptionTableInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported exception_table scenario: {}",
            invocation.scenario
        ));
    }

    let table = ExceptionTable::from_handler_positions(
        vec![
            (ClassRef::from_name("java.lang.Number"), 11),
            (ClassRef::from_name("java.lang.RuntimeException"), 22),
            (ClassRef::from_name("java.lang.Object"), 33),
        ],
        Some(44),
    );
    let object_first = ExceptionTable::from_handler_positions(
        vec![
            (ClassRef::from_name("java.lang.Object"), 5),
            (ClassRef::from_name("java.lang.Number"), 6),
        ],
        None,
    );
    let illegal_argument =
        OpaqueNativeObject::new("java.lang.IllegalArgumentException").into_data_value();
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("null_to_first"),
        table
            .get_relative_pos(&DataValue::Null)
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("integer_to_number"),
        table
            .get_relative_pos(&DataValue::Int(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("long_to_number"),
        table
            .get_relative_pos(&DataValue::Long(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("runtime_subclass"),
        table
            .get_relative_pos(&illegal_argument)
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("string_to_object"),
        table
            .get_relative_pos(&DataValue::string("fallback"))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("final_pos"),
        table
            .get_final_pos()
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("declaration_order"),
        object_first
            .get_relative_pos(&DataValue::Int(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("empty_relative"),
        ExceptionTable::new()
            .get_relative_pos(&DataValue::Int(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("empty_final"),
        ExceptionTable::new()
            .get_final_pos()
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );

    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some(normalize(&DataValue::map(observed))),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

