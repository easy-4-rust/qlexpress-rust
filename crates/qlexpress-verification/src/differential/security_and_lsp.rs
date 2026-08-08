fn execute_security_strategies(
    id: String,
    invocation: SecurityStrategiesInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported security_strategies scenario: {}",
            invocation.scenario
        ));
    }

    let member = NativeMember::new("java.lang.String", "length");
    let other = NativeMember::new("java.lang.String", "substring");
    let mut observed = IndexMap::new();

    let open = StrategyOpen::instance();
    observed.insert(
        DataValue::string("open_singleton"),
        DataValue::Bool(open == StrategyOpen::instance()),
    );
    observed.insert(
        DataValue::string("open_member"),
        DataValue::Bool(open.check(Some(&member))),
    );
    observed.insert(
        DataValue::string("open_null"),
        DataValue::Bool(open.check(None)),
    );

    let isolation = StrategyIsolation::instance();
    observed.insert(
        DataValue::string("isolation_singleton"),
        DataValue::Bool(isolation == StrategyIsolation::instance()),
    );
    observed.insert(
        DataValue::string("isolation_member_error"),
        DataValue::string(
            isolation
                .check(Some(&member))
                .expect_err("isolation always throws")
                .error_code(),
        ),
    );
    observed.insert(
        DataValue::string("isolation_null_error"),
        DataValue::string(
            isolation
                .check(None)
                .expect_err("isolation always throws")
                .error_code(),
        ),
    );

    let black_members = Rc::new(RefCell::new(std::collections::HashSet::new()));
    let black = StrategyBlackList::from_shared(Some(Rc::clone(&black_members)));
    observed.insert(
        DataValue::string("black_empty_member"),
        DataValue::Bool(
            black
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    observed.insert(
        DataValue::string("black_empty_null"),
        DataValue::Bool(black.check(None).map_err(|error| error.to_string())?),
    );
    black_members.borrow_mut().insert(Some(member.clone()));
    black_members.borrow_mut().insert(None);
    observed.insert(
        DataValue::string("black_added_member"),
        DataValue::Bool(
            black
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    observed.insert(
        DataValue::string("black_added_null"),
        DataValue::Bool(black.check(None).map_err(|error| error.to_string())?),
    );

    let white_members = Rc::new(RefCell::new(std::collections::HashSet::new()));
    let white = StrategyWhiteList::from_shared(Some(Rc::clone(&white_members)));
    observed.insert(
        DataValue::string("white_empty_member"),
        DataValue::Bool(
            white
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    observed.insert(
        DataValue::string("white_empty_null"),
        DataValue::Bool(white.check(None).map_err(|error| error.to_string())?),
    );
    white_members.borrow_mut().insert(Some(member.clone()));
    white_members.borrow_mut().insert(None);
    observed.insert(
        DataValue::string("white_added_member"),
        DataValue::Bool(
            white
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    observed.insert(
        DataValue::string("white_added_null"),
        DataValue::Bool(white.check(None).map_err(|error| error.to_string())?),
    );

    let black_null = StrategyBlackList::from_shared(None);
    observed.insert(
        DataValue::string("black_null_set_error"),
        DataValue::string(
            black_null
                .check(Some(&other))
                .expect_err("null Java set must fail")
                .error_code(),
        ),
    );
    let white_null = StrategyWhiteList::from_shared(None);
    observed.insert(
        DataValue::string("white_null_set_error"),
        DataValue::string(
            white_null
                .check(Some(&other))
                .expect_err("null Java set must fail")
                .error_code(),
        ),
    );

    let facade_open = QLSecurityStrategy::open();
    observed.insert(
        DataValue::string("facade_open_impl"),
        DataValue::Bool(facade_open == QLSecurityStrategy::open()),
    );
    observed.insert(
        DataValue::string("facade_open_member"),
        DataValue::Bool(
            facade_open
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    observed.insert(
        DataValue::string("facade_open_null"),
        DataValue::Bool(facade_open.check(None).map_err(|error| error.to_string())?),
    );

    let facade_isolation = QLSecurityStrategy::isolation();
    observed.insert(
        DataValue::string("facade_isolation_impl"),
        DataValue::Bool(facade_isolation == QLSecurityStrategy::isolation()),
    );
    observed.insert(
        DataValue::string("facade_isolation_member_error"),
        DataValue::string(
            facade_isolation
                .check(Some(&member))
                .expect_err("isolation always throws")
                .error_code(),
        ),
    );
    observed.insert(
        DataValue::string("facade_isolation_null_error"),
        DataValue::string(
            facade_isolation
                .check(None)
                .expect_err("isolation always throws")
                .error_code(),
        ),
    );

    let facade_black_members = Rc::new(RefCell::new(std::collections::HashSet::new()));
    let facade_black = QLSecurityStrategy::shared_black_list(Rc::clone(&facade_black_members));
    observed.insert(
        DataValue::string("facade_black_empty"),
        DataValue::Bool(
            facade_black
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    facade_black_members.borrow_mut().insert(member.clone());
    observed.insert(
        DataValue::string("facade_black_added"),
        DataValue::Bool(
            facade_black
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    observed.insert(
        DataValue::string("facade_black_null"),
        DataValue::Bool(
            facade_black
                .check(None)
                .map_err(|error| error.to_string())?,
        ),
    );

    let facade_white_members = Rc::new(RefCell::new(std::collections::HashSet::new()));
    let facade_white = QLSecurityStrategy::shared_white_list(Rc::clone(&facade_white_members));
    observed.insert(
        DataValue::string("facade_white_empty"),
        DataValue::Bool(
            facade_white
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    facade_white_members.borrow_mut().insert(member.clone());
    observed.insert(
        DataValue::string("facade_white_added"),
        DataValue::Bool(
            facade_white
                .check(Some(&member))
                .map_err(|error| error.to_string())?,
        ),
    );
    observed.insert(
        DataValue::string("facade_white_null"),
        DataValue::Bool(
            facade_white
                .check(None)
                .map_err(|error| error.to_string())?,
        ),
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

fn execute_ql_string_utils(
    id: String,
    invocation: QLStringUtilsInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported ql_string_utils scenario: {}",
            invocation.scenario
        ));
    }

    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("standard"),
        DataValue::string(QLStringUtils::parse_string_escape(
            r#""a\nb\t\r\f\b\"\'\\\$z""#,
        )),
    );
    observed.insert(
        DataValue::string("unknown"),
        DataValue::string(QLStringUtils::parse_string_escape(r#""a\xb""#)),
    );
    observed.insert(
        DataValue::string("trailing"),
        DataValue::string(QLStringUtils::parse_string_escape(r#""abc\""#)),
    );
    observed.insert(
        DataValue::string("supplementary"),
        DataValue::string(QLStringUtils::parse_string_escape("\"😀\"")),
    );
    observed.insert(
        DataValue::string("split_high"),
        DataValue::string(QLStringUtils::parse_string_escape_start_end("😀", 0, 1)),
    );
    observed.insert(
        DataValue::string("reverse"),
        DataValue::string(QLStringUtils::parse_string_escape_start_end("a", 3, 2)),
    );
    observed.insert(
        DataValue::string("negative_error"),
        DataValue::string(catch_string_index_error(|| {
            QLStringUtils::parse_string_escape_start_end("a", -1, 1);
        })),
    );
    observed.insert(
        DataValue::string("overflow_error"),
        DataValue::string(catch_string_index_error(|| {
            QLStringUtils::parse_string_escape_start_end("a", 0, 2);
        })),
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

fn catch_string_index_error(action: impl FnOnce()) -> &'static str {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action));
    std::panic::set_hook(previous_hook);
    if result.is_err() {
        "java.lang.StringIndexOutOfBoundsException"
    } else {
        ""
    }
}

fn execute_lsp_range(
    id: String,
    invocation: LspRangeInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported lsp_range scenario: {}",
            invocation.scenario
        ));
    }

    let range = Range::new(Position::new(1, 2), Position::new(3, 4));
    let nullable = Range::from_options(None, None);
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("start_line"),
        range
            .start()
            .map(|position| DataValue::Int(position.line()))
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("start_character"),
        range
            .start()
            .map(|position| DataValue::Int(position.character()))
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("end_line"),
        range
            .end()
            .map(|position| DataValue::Int(position.line()))
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("end_character"),
        range
            .end()
            .map(|position| DataValue::Int(position.character()))
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("null_start"),
        nullable
            .start()
            .map(|_| DataValue::Bool(false))
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("null_end"),
        nullable
            .end()
            .map(|_| DataValue::Bool(false))
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

fn execute_lsp_diagnostic(
    id: String,
    invocation: LspDiagnosticInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported lsp_diagnostic scenario: {}",
            invocation.scenario
        ));
    }

    let diagnostic = Diagnostic::new(
        12,
        Range::new(Position::new(1, 2), Position::new(1, 5)),
        "abc",
        "E001",
        "bad input",
        "a = abc",
    );
    let nullable = Diagnostic::from_options(-5, None, None, None, None, None);
    let mut observed = IndexMap::new();
    observed.insert(DataValue::string("pos"), DataValue::Int(diagnostic.pos()));
    observed.insert(
        DataValue::string("range_start_line"),
        diagnostic
            .range()
            .and_then(Range::start)
            .map(|position| DataValue::Int(position.line()))
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("lexeme"),
        optional_string(diagnostic.lexeme()),
    );
    observed.insert(
        DataValue::string("code"),
        optional_string(diagnostic.code()),
    );
    observed.insert(
        DataValue::string("message"),
        optional_string(diagnostic.message()),
    );
    observed.insert(
        DataValue::string("snippet"),
        optional_string(diagnostic.snippet()),
    );
    observed.insert(
        DataValue::string("nullable_pos"),
        DataValue::Int(nullable.pos()),
    );
    observed.insert(
        DataValue::string("nullable_range"),
        nullable
            .range()
            .map(|_| DataValue::Bool(false))
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("nullable_lexeme"),
        optional_string(nullable.lexeme()),
    );
    observed.insert(
        DataValue::string("nullable_code"),
        optional_string(nullable.code()),
    );
    observed.insert(
        DataValue::string("nullable_message"),
        optional_string(nullable.message()),
    );
    observed.insert(
        DataValue::string("nullable_snippet"),
        optional_string(nullable.snippet()),
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

fn optional_string(value: Option<&str>) -> DataValue {
    value.map(DataValue::string).unwrap_or(DataValue::Null)
}

