//! 展开方法调用指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.SpreadMethodInvokeInstruction`。
//! 职责:对集合元素逐个调用方法并收集结果。
//! 本文件由 `call.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::{find_method_and_invoke, invoke_native_method};
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// 展开方法调用指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.SpreadMethodInvokeInstruction(职责:对集合元素逐个调用方法并收集结果)
/// Operation: Invoke specified method of each object in the list
/// Input: ${argNum} + 1
/// Output: 1, a list composed of return values from methods.
///
/// Mirrors Java `SpreadMethodInvokeInstruction`.
pub struct SpreadMethodInvokeInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    method_name: String,
    arg_num: usize,
}

impl SpreadMethodInvokeInstruction {
    /// 构造指令,对应 Java 构造器 `SpreadMethodInvokeInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        method_name: impl Into<String>,
        arg_num: usize,
    ) -> Self {
        SpreadMethodInvokeInstruction {
            error_reporter,
            method_name: method_name.into(),
            arg_num,
        }
    }

    /// 对应 Java 方法 `methodName`。
    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    /// 对应 Java 方法 `argNum`。
    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    /// Java `isTraversable` (Iterable or array → List/Array here).
    fn is_traversable(obj: &DataValue) -> bool {
        matches!(obj, DataValue::List(_) | DataValue::Array(_))
    }

    /// Java `spreadMethodInvokeRecursive`.
    fn spread_recursive(
        &self,
        traversable: &DataValue,
        params: &[DataValue],
        q_context: &dyn QContext,
        ql_options: &QLOptions,
        result: &mut Vec<DataValue>,
    ) -> Result<(), QLException> {
        let items = match traversable {
            DataValue::List(l) => l.borrow().clone(),
            DataValue::Array(a) => a.borrow().clone(),
            _ => vec![],
        };
        for item in items {
            self.process_item(&item, params, q_context, ql_options, result)?;
        }
        Ok(())
    }

    /// Java `processItem`.
    fn process_item(
        &self,
        item: &DataValue,
        params: &[DataValue],
        q_context: &dyn QContext,
        ql_options: &QLOptions,
        result: &mut Vec<DataValue>,
    ) -> Result<(), QLException> {
        if item.is_null() {
            if ql_options.is_avoid_null_pointer() {
                result.push(DataValue::Null);
                return Ok(());
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_METHOD_ACCESS,
                error_codes::error_msg(error_codes::NULL_METHOD_ACCESS),
            ));
        }

        if !Self::is_traversable(item) {
            // Leaf node - invoke method directly
            let invoke_res = find_method_and_invoke(
                item,
                &self.method_name,
                params,
                q_context.registry(),
                &*self.error_reporter,
            )?;
            result.push(invoke_res.get());
            return Ok(());
        }
        // If item itself is traversable, try to invoke method on it first
        if let Some(method) = q_context.registry().resolve_method(item, &self.method_name) {
            let invoke_res = invoke_native_method(item, &method, params)?;
            result.push(invoke_res.get());
            return Ok(());
        }
        // Then recursively flatten and invoke on nested elements
        self.spread_recursive(item, params, q_context, ql_options, result)
    }
}

impl QLInstruction for SpreadMethodInvokeInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let parameters = q_context.pop_n(self.arg_num + 1);
        let traversable = parameters.get(0).expect("bean slot popped").get();
        if traversable.is_null() {
            if ql_options.is_avoid_null_pointer() {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report_format(
                error_codes::NONTRAVERSABLE_OBJECT,
                error_codes::error_msg(error_codes::NONTRAVERSABLE_OBJECT),
                &["null".to_string()],
            ));
        }
        let params: Vec<DataValue> = (0..self.arg_num)
            .map(|i| parameters.get_value(i + 1))
            .collect();

        if Self::is_traversable(&traversable) {
            let mut result = Vec::new();
            self.spread_recursive(&traversable, &params, q_context, ql_options, &mut result)?;
            q_context.push(QValue::Data(DataValue::list(result)));
        } else {
            return Err(self.error_reporter.report_format(
                error_codes::NONTRAVERSABLE_OBJECT,
                error_codes::error_msg(error_codes::NONTRAVERSABLE_OBJECT),
                &[traversable.data_type_name().to_string()],
            ));
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32 + 1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: SpreadMethodInvoke {}", index, self.method_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

