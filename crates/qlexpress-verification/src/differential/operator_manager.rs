fn execute_operator_manager(
    id: String,
    invocation: OperatorManagerInvocation,
) -> Result<DifferentialRecord, String> {
    let mut manager = OperatorManager::new();
    for setup in &invocation.setup {
        if !apply_operator_manager_setup(&mut manager, setup)? {
            return Err(format!(
                "operator_manager setup failed: {} {}",
                setup.action, setup.lexeme
            ));
        }
    }

    let lexeme = || {
        invocation
            .lexeme
            .as_deref()
            .ok_or_else(|| format!("operator_manager {} requires lexeme", invocation.operation))
    };
    let normalized =
        match invocation.operation.as_str() {
            "addBinaryOperator" => normalize(&DataValue::Bool(manager.add_binary_operator(
                lexeme()?,
                additive_custom_operator(),
                invocation.priority.unwrap_or(300),
            ))),
            "replaceDefaultOperator" => normalize(&DataValue::Bool(
                manager.replace_default_operator(lexeme()?, additive_custom_operator()),
            )),
            "addOperatorAlias" => normalize(&DataValue::Bool(manager.add_operator_alias(
                lexeme()?,
                invocation.origin.as_deref().ok_or_else(|| {
                    "operator_manager addOperatorAlias requires origin".to_string()
                })?,
            ))),
            "addKeyWordAlias" => normalize(&DataValue::Bool(manager.add_key_word_alias(
                lexeme()?,
                invocation.keyword.as_deref().ok_or_else(|| {
                    "operator_manager addKeyWordAlias requires keyword".to_string()
                })?,
            ))),
            "getBinaryOperator" => operator_metadata(manager.get_binary_operator(lexeme()?)),
            "getPrefixUnaryOperator" => manager
                .get_prefix_unary_operator(lexeme()?)
                .map(|operator| {
                    normalize(&DataValue::string(format!(
                        "{}|{}",
                        operator.operator(),
                        operator.priority()
                    )))
                })
                .unwrap_or_else(|| normalize(&DataValue::Null)),
            "getSuffixUnaryOperator" => manager
                .get_suffix_unary_operator(lexeme()?)
                .map(|operator| {
                    normalize(&DataValue::string(format!(
                        "{}|{}",
                        operator.operator(),
                        operator.priority()
                    )))
                })
                .unwrap_or_else(|| normalize(&DataValue::Null)),
            "isOpType" => {
                let op_type = match invocation.op_type.as_deref() {
                    Some("MIDDLE") => OpType::Middle,
                    Some("PREFIX") => OpType::Prefix,
                    Some("SUFFIX") => OpType::Suffix,
                    value => {
                        return Err(format!("unsupported operator_manager op_type: {value:?}"))
                    }
                };
                normalize(&DataValue::Bool(manager.is_op_type(lexeme()?, op_type)))
            }
            "precedence" => {
                let lexeme = lexeme()?;
                let previous_hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(|_| {}));
                let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    manager.precedence(lexeme)
                }));
                std::panic::set_hook(previous_hook);
                match observed {
                    Ok(precedence) => precedence
                        .map(DataValue::Int)
                        .map(|value| normalize(&value))
                        .unwrap_or_else(|| normalize(&DataValue::Null)),
                    Err(payload) => {
                        let reason = payload
                            .downcast_ref::<&str>()
                            .copied()
                            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                            .unwrap_or_default();
                        return Ok(DifferentialRecord {
                            id,
                            outcome: "error",
                            normalized: Some(format!("error:NullPointerException:{reason}")),
                            error_code: Some("NullPointerException".to_string()),
                            line: Some(0),
                            column: Some(0),
                            trace_count: 0,
                        });
                    }
                }
            }
            "getAlias" => manager
                .get_alias(lexeme()?)
                .map(DataValue::Int)
                .map(|value| normalize(&value))
                .unwrap_or_else(|| normalize(&DataValue::Null)),
            "executeBinary" => {
                let left =
                    typed_number_to_data_value(invocation.left.as_ref().ok_or_else(|| {
                        "operator_manager executeBinary requires left".to_string()
                    })?)?;
                let right =
                    typed_number_to_data_value(invocation.right.as_ref().ok_or_else(|| {
                        "operator_manager executeBinary requires right".to_string()
                    })?)?;
                let operator_lexeme = lexeme()?;
                let operator = manager
                    .get_binary_operator(operator_lexeme)
                    .ok_or_else(|| {
                        format!("operator_manager binary operator not found: {operator_lexeme}")
                    })?;
                let runtime = QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins()));
                let global_scope = QScope::global(QvmGlobalScope::empty());
                let instruction_scope =
                    QScope::block_fresh_stack(&global_scope, Default::default(), 4);
                let mut context = DelegateQContext::new(Rc::new(runtime), instruction_scope);
                match operator.execute(
                    &QValue::Data(left),
                    &QValue::Data(right),
                    &mut context,
                    &QLOptions::builder().build(),
                    &PureErrReporter::INSTANCE,
                ) {
                    Ok(value) => normalize(&value),
                    Err(error) => {
                        return Ok(DifferentialRecord {
                            id,
                            outcome: "error",
                            normalized: Some(format!(
                                "error:{}:{}",
                                error.error_code(),
                                error.reason()
                            )),
                            error_code: Some(error.error_code().to_string()),
                            line: Some(error.line_no()),
                            column: Some(error.col_no()),
                            trace_count: 0,
                        });
                    }
                }
            }
            operation => {
                return Err(format!(
                    "unsupported operator_manager operation: {operation}"
                ))
            }
        };

    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some(normalized),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

fn additive_custom_operator() -> Rc<dyn qlexpress::operator::CustomBinaryOperator> {
    Rc::new(AdditiveCustomOperator)
}

struct AdditiveCustomOperator;

impl qlexpress::operator::CustomBinaryOperator for AdditiveCustomOperator {
    #[allow(clippy::result_large_err)]
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
    ) -> Result<DataValue, qlexpress::exception::QLException> {
        let left = left.get();
        let right = right.get();
        NumberMath::add(&left, &right)
    }
}

fn apply_operator_manager_setup(
    manager: &mut OperatorManager,
    setup: &OperatorManagerSetup,
) -> Result<bool, String> {
    match setup.action.as_str() {
        "add" => Ok(manager.add_binary_operator(
            &setup.lexeme,
            additive_custom_operator(),
            setup.priority.unwrap_or(300),
        )),
        "replace" => {
            Ok(manager.replace_default_operator(&setup.lexeme, additive_custom_operator()))
        }
        "operator_alias" => Ok(manager.add_operator_alias(
            &setup.lexeme,
            setup.origin.as_deref().ok_or_else(|| {
                "operator_manager operator_alias setup requires origin".to_string()
            })?,
        )),
        "keyword_alias" => Ok(manager.add_key_word_alias(
            &setup.lexeme,
            setup.keyword.as_deref().ok_or_else(|| {
                "operator_manager keyword_alias setup requires keyword".to_string()
            })?,
        )),
        action => Err(format!(
            "unsupported operator_manager setup action: {action}"
        )),
    }
}

fn operator_metadata(operator: Option<Rc<dyn qlexpress::operator::BinaryOperator>>) -> String {
    operator
        .map(|operator| {
            normalize(&DataValue::string(format!(
                "{}|{}",
                operator.operator(),
                operator.priority()
            )))
        })
        .unwrap_or_else(|| normalize(&DataValue::Null))
}

