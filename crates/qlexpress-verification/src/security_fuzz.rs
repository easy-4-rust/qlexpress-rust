//! 可重复的安全语法 fuzz 与隔离策略不变量测试。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

use qlexpress_rust::exception::ql_exception::QLException;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::native_object::NativeObject;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress_rust::Express4Runner;

const TOKENS: &[&str] = &[
    "a",
    "b",
    "null",
    "true",
    "false",
    "0",
    "1",
    "-1",
    "2147483648L",
    "'x'",
    "\"${a}\"",
    "+",
    "-",
    "*",
    "/",
    "%",
    "==",
    "!=",
    "&&",
    "||",
    "?",
    ":",
    "=",
    ".",
    "::",
    "[",
    "]",
    "(",
    ")",
    "{",
    "}",
    ",",
    ";",
    "if",
    "else",
    "for",
    "while",
    "return",
    "new",
    "throw",
    "sensitive",
    "secret",
    "exec",
    "class",
    "getClass",
];

struct SensitiveHost {
    invocations: Rc<Cell<usize>>,
}

impl NativeObject for SensitiveHost {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        self.invocations.set(self.invocations.get() + 1);
        Some(DataValue::Str("secret".to_string()))
    }

    fn call_method(&mut self, _name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        self.invocations.set(self.invocations.get() + 1);
        Ok(DataValue::Str("executed".to_string()))
    }

    fn native_type_name(&self) -> &str {
        "com.example.SensitiveHost"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn run(cases: usize) -> Result<(), String> {
    if cases == 0 {
        return Err("fuzz cases must be greater than zero".to_string());
    }
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::isolation())
            .build(),
    );
    let options = QLOptions::builder()
        .timeout_millis(5)
        .max_arr_length(1_024)
        .build();
    let invocations = Rc::new(Cell::new(0));
    let host: Rc<RefCell<dyn NativeObject>> = Rc::new(RefCell::new(SensitiveHost {
        invocations: Rc::clone(&invocations),
    }));
    let mut state = 0x6a09_e667_f3bc_c909u64;
    for case_index in 0..cases {
        let token_count = 1 + (next(&mut state) as usize % 48);
        let mut script = String::new();
        for _ in 0..token_count {
            if !script.is_empty() {
                script.push(' ');
            }
            script.push_str(TOKENS[next(&mut state) as usize % TOKENS.len()]);
        }
        let mut context = HashMap::new();
        context.insert("sensitive".to_string(), DataValue::Object(Rc::clone(&host)));
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let _ = runner.execute(&script, context, &options);
        }));
        if outcome.is_err() {
            return Err(format!(
                "panic at fuzz case {case_index}, script: {}",
                script.replace('\n', "\\n")
            ));
        }
        if invocations.get() != 0 {
            return Err(format!(
                "isolation bypass at fuzz case {case_index}, script: {script}"
            ));
        }
    }
    println!(
        "{{\"seed\":\"0x6a09e667f3bcc909\",\"cases\":{cases},\"panics\":0,\"isolation_bypasses\":0}}"
    );
    Ok(())
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
