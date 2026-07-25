//! String-join instruction, mirroring Java `StringJoinInstruction`.

use std::rc::Rc;

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Operation: concat n string on the top of stack
/// Input: ${n}
/// Output: concat result
///
/// Mirrors Java `StringJoinInstruction`.
pub struct StringJoinInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    n: usize,
}

impl StringJoinInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, n: usize) -> Self {
        StringJoinInstruction { error_reporter, n }
    }

    pub fn n(&self) -> usize {
        self.n
    }
}

impl QLInstruction for StringJoinInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let arguments = q_context.pop_n(self.n);
        let mut sb = String::new();
        for i in 0..self.n {
            // Java StringBuilder.append(Object) → String.valueOf
            sb.push_str(&arguments.get_value(i).string_value_of());
        }
        q_context.push(QValue::Data(DataValue::Str(sb)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.n as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: StringJoin {}", index, self.n),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
