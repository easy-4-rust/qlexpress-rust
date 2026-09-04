fn execute_delegate_context(
    id: String,
    invocation: DelegateContextInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario == "close_global" {
        return execute_delegate_close_global(id);
    }
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported delegate_context scenario: {}",
            invocation.scenario
        ));
    }

    let mut attachment_values = Attachments::new();
    attachment_values.insert("tenant".to_string(), DataValue::string("acme"));
    let shared_attachments: SharedAttachments = Rc::new(std::cell::RefCell::new(attachment_values));
    let registry = Rc::new(NativeRegistry::with_builtins());
    let runtime = Rc::new(QvmRuntime::new(
        QTraces::empty(),
        Rc::clone(&shared_attachments),
        Rc::clone(&registry),
        123_456,
    ));
    let shared_functions = Rc::new(std::cell::RefCell::new(HashMap::new()));
    let global_scope = QScope::global(QvmGlobalScope::with_shared_context(
        Rc::new(EmptyContext::new()),
        Rc::clone(&shared_functions),
        Rc::clone(&shared_attachments),
        false,
    ));
    let block_scope = QScope::block_fresh_stack(&global_scope, Default::default(), 8);
    let mut context = DelegateQContext::new(Rc::clone(&runtime), Rc::clone(&block_scope));
    let mut observed = IndexMap::new();

    observed.insert(
        DataValue::string("start_time"),
        DataValue::Long(context.script_start_time_stamp()),
    );
    observed.insert(
        DataValue::string("attachment"),
        context
            .attachment()
            .get("tenant")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("reflect_same"),
        DataValue::Bool(Rc::ptr_eq(context.get_reflect_loader(), &registry)),
    );
    observed.insert(
        DataValue::string("traces_same"),
        DataValue::Bool(std::ptr::eq(context.traces(), runtime.traces())),
    );
    observed.insert(
        DataValue::string("trace_count"),
        DataValue::Int(context.traces().snapshot().len() as i32),
    );
    observed.insert(
        DataValue::string("current_initial"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
    );
    observed.insert(
        DataValue::string("parent_initial"),
        DataValue::Bool(
            context
                .parent_scope()
                .is_some_and(|parent| Rc::ptr_eq(&parent, &global_scope)),
        ),
    );

    context.define_local_symbol(
        "x",
        Some(ClassRef::from_name("java.lang.Integer")),
        DataValue::Int(7),
    );
    let symbol = context
        .get_symbol("x")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "DelegateQContext lost local symbol x".to_string())?;
    observed.insert(DataValue::string("symbol_present"), DataValue::Bool(true));
    observed.insert(DataValue::string("symbol_value"), symbol.borrow().get());
    observed.insert(
        DataValue::string("missing_value"),
        context
            .get_symbol_value("missing")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );

    let function = delegate_contract_function();
    context.define_function("f", Rc::clone(&function));
    observed.insert(
        DataValue::string("function_get"),
        DataValue::Bool(context.get_function("f").is_some()),
    );
    let function_table = context.function_table();
    function_table
        .borrow_mut()
        .insert("g".to_string(), Rc::clone(&function));
    observed.insert(
        DataValue::string("function_table_size"),
        DataValue::Int(function_table.borrow().len() as i32),
    );
    observed.insert(
        DataValue::string("function_table_write_through"),
        DataValue::Bool(context.get_function("g").is_some()),
    );

    context.push(DataValue::Int(1).into());
    context.push(DataValue::Int(2).into());
    let child_scope = context.new_scope();
    context.push(DataValue::Int(3).into());
    observed.insert(DataValue::string("stack_peek"), context.peek().get());
    let popped = context.pop_n(2);
    observed.insert(
        DataValue::string("pop_n_size"),
        DataValue::Int(popped.size() as i32),
    );
    observed.insert(DataValue::string("pop_n_0"), popped.get_value(0));
    observed.insert(DataValue::string("pop_n_1"), popped.get_value(1));
    observed.insert(DataValue::string("stack_after_pop_n"), context.peek().get());
    context.push(DataValue::Int(4).into());
    observed.insert(DataValue::string("pop_single"), context.pop().get());
    observed.insert(
        DataValue::string("child_current"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &child_scope)),
    );
    observed.insert(
        DataValue::string("child_parent"),
        DataValue::Bool(
            context
                .parent_scope()
                .is_some_and(|parent| Rc::ptr_eq(&parent, &block_scope)),
        ),
    );
    observed.insert(
        DataValue::string("child_inherits_function"),
        DataValue::Bool(context.get_function("f").is_some()),
    );
    observed.insert(
        DataValue::string("child_function_table_size"),
        DataValue::Int(context.function_table().borrow().len() as i32),
    );
    observed.insert(
        DataValue::string("child_inherited_symbol"),
        context
            .get_symbol_value("x")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );
    context.define_local_symbol(
        "x",
        Some(ClassRef::from_name("java.lang.Integer")),
        DataValue::Int(9),
    );
    observed.insert(
        DataValue::string("child_shadow"),
        context
            .get_symbol_value("x")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );
    context.close_scope();
    observed.insert(
        DataValue::string("closed_to_parent"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
    );
    observed.insert(
        DataValue::string("parent_symbol_after_close"),
        context
            .get_symbol_value("x")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );
    context.close_scope();
    observed.insert(
        DataValue::string("closed_to_global"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &global_scope)),
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

fn execute_delegate_close_global(id: String) -> Result<DifferentialRecord, String> {
    let registry = Rc::new(NativeRegistry::with_builtins());
    let runtime = Rc::new(QvmRuntime::for_test(registry));
    let mut context = DelegateQContext::new(runtime, QScope::global(QvmGlobalScope::empty()));
    let global_scope = Rc::clone(&context.current_scope());
    // close_scope on root scope should be a no-op (no panic).
    context.close_scope();
    if !Rc::ptr_eq(&context.current_scope(), &global_scope) {
        return Err("DelegateQContext.close_scope changed scope on root (expected no-op)".to_string());
    }
    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some("close_global_noop:true".to_string()),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

fn delegate_contract_function() -> Rc<dyn qlexpress::runtime::CustomFunction> {
    Rc::new(DelegateContractFunction)
}

struct DelegateContractFunction;

impl qlexpress::runtime::CustomFunction for DelegateContractFunction {
    #[allow(clippy::result_large_err)]
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &qlexpress::runtime::parameters::Parameters,
    ) -> Result<DataValue, qlexpress::exception::QLException> {
        Ok(DataValue::Int(parameters.size() as i32))
    }
}

