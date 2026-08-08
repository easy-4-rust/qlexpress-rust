fn execute_batch_add_function_result(
    id: String,
    invocation: BatchAddFunctionResultInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported batch_add_function_result scenario: {}",
            invocation.scenario
        ));
    }

    let mut result = BatchAddFunctionResult::new();
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("initial_succ"),
        string_list(result.succ()),
    );
    observed.insert(
        DataValue::string("initial_fail"),
        string_list(result.fail()),
    );
    observed.insert(
        DataValue::string("initial_all_succ"),
        DataValue::Bool(result.is_all_succ()),
    );

    result.succ_mut().push("external-success".to_string());
    result.fail_mut().push("external-failure".to_string());
    result.add_succ("runner-success");
    result.add_fail("runner-failure");

    observed.insert(DataValue::string("succ"), string_list(result.succ()));
    observed.insert(DataValue::string("fail"), string_list(result.fail()));
    observed.insert(
        DataValue::string("all_succ_after_failure"),
        DataValue::Bool(result.is_all_succ()),
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

fn string_list(values: &[String]) -> DataValue {
    DataValue::list(
        values
            .iter()
            .map(|value| DataValue::string(value.clone()))
            .collect(),
    )
}

// QLFunctionalVarargs 的迁移契约固定返回 QLException；本差分函数必须直接
// 构造该 trait 闭包，不能为了测试规避而改用较小的非契约错误类型。
#[allow(clippy::result_large_err)]
fn execute_ql_functional_varargs(
    id: String,
    invocation: QLFunctionalVarargsInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported ql_functional_varargs scenario: {}",
            invocation.scenario
        ));
    }

    let count = |params: &[DataValue]| {
        Ok::<DataValue, qlexpress::exception::QLException>(DataValue::Int(params.len() as i32))
    };
    let collect = |params: &[DataValue]| {
        Ok::<DataValue, qlexpress::exception::QLException>(DataValue::list(params.to_vec()))
    };
    let returns_null =
        |_params: &[DataValue]| Ok::<DataValue, qlexpress::exception::QLException>(DataValue::Null);
    let parameters = [DataValue::Int(1), DataValue::string("x"), DataValue::Null];
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("empty_count"),
        QLFunctionalVarargs::call(&count, &[]).map_err(|error| error.to_string())?,
    );
    observed.insert(
        DataValue::string("ordered_values"),
        QLFunctionalVarargs::call(&collect, &parameters).map_err(|error| error.to_string())?,
    );
    observed.insert(
        DataValue::string("null_result"),
        QLFunctionalVarargs::call(&returns_null, &parameters).map_err(|error| error.to_string())?,
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

fn execute_lsp_position(
    id: String,
    invocation: LspPositionInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported lsp_position scenario: {}",
            invocation.scenario
        ));
    }

    let normal = Position::new(7, 99);
    let negative = Position::new(-1, -2);
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("normal_line"),
        DataValue::Int(normal.line()),
    );
    observed.insert(
        DataValue::string("normal_character"),
        DataValue::Int(normal.character()),
    );
    observed.insert(
        DataValue::string("negative_line"),
        DataValue::Int(negative.line()),
    );
    observed.insert(
        DataValue::string("negative_character"),
        DataValue::Int(negative.character()),
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

fn execute_exist_stack(
    id: String,
    invocation: ExistStackInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported exist_stack scenario: {}",
            invocation.scenario
        ));
    }

    let mut root = ExistVarStack::root();
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("initial_root"),
        DataValue::Bool(root.exist(Some("root"))),
    );
    observed.insert(
        DataValue::string("initial_null"),
        DataValue::Bool(root.exist(None)),
    );
    observed.insert(
        DataValue::string("root_pop_is_null"),
        DataValue::Bool(root.pop().is_none()),
    );

    root.add(Some("root".to_string()));
    root.add(Some("root".to_string()));
    root.add(None);
    observed.insert(
        DataValue::string("root_after_add"),
        DataValue::Bool(root.exist(Some("root"))),
    );
    observed.insert(
        DataValue::string("null_after_add"),
        DataValue::Bool(root.exist(None)),
    );

    let mut child = root.push();
    observed.insert(
        DataValue::string("child_sees_root"),
        DataValue::Bool(child.exist(Some("root"))),
    );
    child.add(Some("child".to_string()));
    observed.insert(
        DataValue::string("child_local"),
        DataValue::Bool(child.exist(Some("child"))),
    );
    let parent = child
        .pop()
        .ok_or_else(|| "child stack lost its parent".to_string())?;
    observed.insert(
        DataValue::string("parent_has_root"),
        DataValue::Bool(parent.exist(Some("root"))),
    );
    observed.insert(
        DataValue::string("parent_has_child"),
        DataValue::Bool(parent.exist(Some("child"))),
    );
    observed.insert(
        DataValue::string("parent_has_null"),
        DataValue::Bool(parent.exist(None)),
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

fn execute_macro_define(
    id: String,
    invocation: MacroDefineInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported macro_define scenario: {}",
            invocation.scenario
        ));
    }

    let shared = Rc::new(RefCell::new(vec!["first".to_string()]));
    let define = MacroDefine::from_shared(Some(Rc::clone(&shared)), false);
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("initial_size"),
        DataValue::Int(
            define
                .macro_instructions()
                .expect("non-null instructions")
                .len() as i32,
        ),
    );
    observed.insert(
        DataValue::string("same_instance"),
        DataValue::Bool(Rc::strong_count(&shared) == 2),
    );
    shared.borrow_mut().push("external".to_string());
    observed.insert(
        DataValue::string("external_mutation_size"),
        DataValue::Int(
            define
                .macro_instructions()
                .expect("non-null instructions")
                .len() as i32,
        ),
    );
    define
        .macro_instructions_mut()
        .expect("non-null instructions")
        .push("getter".to_string());
    observed.insert(
        DataValue::string("getter_mutation_size"),
        DataValue::Int(shared.borrow().len() as i32),
    );
    observed.insert(
        DataValue::string("last_stmt_express"),
        DataValue::Bool(define.is_last_stmt_express()),
    );

    let null_define = MacroDefine::<String>::from_shared(None, true);
    observed.insert(
        DataValue::string("null_list"),
        DataValue::Bool(null_define.macro_instructions().is_none()),
    );
    observed.insert(
        DataValue::string("null_last_stmt_express"),
        DataValue::Bool(null_define.is_last_stmt_express()),
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

fn execute_user_define_exception(
    id: String,
    invocation: UserDefineExceptionInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported user_define_exception scenario: {}",
            invocation.scenario
        ));
    }

    let default_error = UserDefineException::new("business");
    let explicit_error = UserDefineException::with_type(ExceptionType::InvalidArgument, "argument");
    let null_error = UserDefineException::from_options(None, None);
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("default_type"),
        optional_exception_type(default_error.get_type()),
    );
    observed.insert(
        DataValue::string("default_message"),
        optional_string(default_error.message()),
    );
    observed.insert(
        DataValue::string("explicit_type"),
        optional_exception_type(explicit_error.get_type()),
    );
    observed.insert(
        DataValue::string("explicit_message"),
        optional_string(explicit_error.message()),
    );
    observed.insert(
        DataValue::string("null_type"),
        optional_exception_type(null_error.get_type()),
    );
    observed.insert(
        DataValue::string("null_message"),
        optional_string(null_error.message()),
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

fn optional_exception_type(value: Option<ExceptionType>) -> DataValue {
    match value {
        Some(ExceptionType::InvalidArgument) => DataValue::string("INVALID_ARGUMENT"),
        Some(ExceptionType::BizException) => DataValue::string("BIZ_EXCEPTION"),
        None => DataValue::Null,
    }
}

