fn execute_runtime_core(
    id: String,
    invocation: RuntimeCoreInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported runtime_core scenario: {}",
            invocation.scenario
        ));
    }

    let mut attachments = Attachments::new();
    attachments.insert("tenant".to_string(), DataValue::string("acme"));
    let shared_attachments: SharedAttachments = Rc::new(std::cell::RefCell::new(attachments));
    let registry = Rc::new(NativeRegistry::with_builtins());
    let traces = QTraces::empty();
    let runtime = Rc::new(QvmRuntime::new(
        traces,
        Rc::clone(&shared_attachments),
        Rc::clone(&registry),
        424_242,
    ));
    let mut observed = IndexMap::new();

    observed.insert(
        DataValue::string("runtime_start"),
        DataValue::Long(runtime.script_start_time_stamp()),
    );
    observed.insert(
        DataValue::string("runtime_attachment_initial"),
        runtime
            .attachment()
            .get("tenant")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("runtime_registry_same"),
        DataValue::Bool(Rc::ptr_eq(runtime.registry(), &registry)),
    );
    observed.insert(
        DataValue::string("runtime_trace_count"),
        DataValue::Int(runtime.traces().snapshot().len() as i32),
    );

    shared_attachments
        .borrow_mut()
        .insert("external_write".to_string(), DataValue::Int(7));
    observed.insert(
        DataValue::string("external_write_visible"),
        runtime
            .attachment()
            .get("external_write")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    runtime
        .attachment_mut()
        .insert("runtime_write".to_string(), DataValue::Int(8));
    observed.insert(
        DataValue::string("runtime_write_visible_external"),
        shared_attachments
            .borrow()
            .get("runtime_write")
            .cloned()
            .unwrap_or(DataValue::Null),
    );

    let global_scope = QScope::global(QvmGlobalScope::empty());
    let block_scope = QScope::block_fresh_stack(&global_scope, Default::default(), 1);
    let mut context = DelegateQContext::new(Rc::clone(&runtime), Rc::clone(&block_scope));
    observed.insert(
        DataValue::string("context_runtime_same"),
        DataValue::Bool(Rc::ptr_eq(context.q_runtime(), &runtime)),
    );
    observed.insert(
        DataValue::string("context_start"),
        DataValue::Long(context.script_start_time_stamp()),
    );
    observed.insert(
        DataValue::string("context_registry_same"),
        DataValue::Bool(Rc::ptr_eq(context.registry(), &registry)),
    );
    observed.insert(
        DataValue::string("context_traces_same"),
        DataValue::Bool(std::ptr::eq(context.traces(), runtime.traces())),
    );
    observed.insert(
        DataValue::string("context_current_initial"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
    );
    context
        .attachment_mut()
        .insert("context_write".to_string(), DataValue::Int(9));
    observed.insert(
        DataValue::string("context_write_visible_runtime"),
        runtime
            .attachment()
            .get("context_write")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    let child = context.new_scope();
    observed.insert(
        DataValue::string("context_current_child"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &child)),
    );
    context.close_scope();
    observed.insert(
        DataValue::string("context_closed_to_parent"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
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

fn execute_fixed_size_stack(
    id: String,
    invocation: FixedSizeStackInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported fixed_size_stack scenario: {}",
            invocation.scenario
        ));
    }

    let mut stack = qlexpress::runtime::fixed_size_stack::FixedSizeStack::new(4);
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("capacity"),
        DataValue::Int(stack.capacity() as i32),
    );
    for value in 1..=4 {
        stack.push(DataValue::Int(value).into());
    }
    observed.insert(DataValue::string("peak"), stack.peak().get());
    observed.insert(DataValue::string("pop_4"), stack.pop().get());
    observed.insert(DataValue::string("pop_3"), stack.pop().get());
    stack.push(DataValue::Int(5).into());
    stack.push(DataValue::Int(6).into());

    let parameters = stack.pop_n(3);
    observed.insert(
        DataValue::string("parameters_size"),
        DataValue::Int(parameters.size() as i32),
    );
    observed.insert(
        DataValue::string("parameters_present_0"),
        DataValue::Bool(parameters.get(0).is_some()),
    );
    observed.insert(
        DataValue::string("parameters_values"),
        DataValue::list(parameters.values()),
    );
    observed.insert(
        DataValue::string("parameters_oob_present"),
        DataValue::Bool(parameters.get(3).is_some()),
    );
    observed.insert(
        DataValue::string("parameters_oob_value"),
        parameters.get_value(3),
    );
    observed.insert(DataValue::string("remaining_peak"), stack.peak().get());

    stack.push(DataValue::Int(9).into());
    observed.insert(
        DataValue::string("live_after_one_push"),
        DataValue::list(parameters.values()),
    );
    stack.push(DataValue::Int(8).into());
    observed.insert(
        DataValue::string("live_after_two_pushes"),
        DataValue::list(parameters.values()),
    );
    stack.push(DataValue::Int(7).into());
    observed.insert(
        DataValue::string("live_after_three_pushes"),
        DataValue::list(parameters.values()),
    );
    observed.insert(DataValue::string("pop_reused_top"), stack.pop().get());
    observed.insert(DataValue::string("peak_after_pop"), stack.peak().get());
    let empty_parameters = stack.pop_n(0);
    observed.insert(
        DataValue::string("zero_pop_size"),
        DataValue::Int(empty_parameters.size() as i32),
    );
    observed.insert(
        DataValue::string("zero_pop_get"),
        DataValue::Bool(empty_parameters.get(0).is_some()),
    );

    let mut null_stack = qlexpress::runtime::fixed_size_stack::FixedSizeStack::new(1);
    null_stack.push(DataValue::Null.into());
    let null_parameters = null_stack.pop_n(1);
    observed.insert(
        DataValue::string("null_slot_present"),
        DataValue::Bool(null_parameters.get(0).is_some()),
    );
    observed.insert(
        DataValue::string("null_slot_value"),
        null_parameters.get_value(0),
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

