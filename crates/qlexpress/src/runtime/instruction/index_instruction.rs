//! 下标访问指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.IndexInstruction`。
//! 职责:按下标读写集合/数组元素。
//! 本文件由 `index.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::data::{ArrayItemValue, ListItemValue, MapItemValue};
use crate::runtime::instruction::QLInstruction;
use crate::runtime::qcontext::QContext;
use crate::runtime::util::value_utils::{assert_number, java_index};
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// 下标访问指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.IndexInstruction(职责:按下标读写集合/数组元素)
/// Operation: extract value with index, like a[0], m['a']
/// Input: 2, indexable object and index
/// Output: 1
///
/// Mirrors Java `IndexInstruction`.
pub struct IndexInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl IndexInstruction {
    /// 构造指令,对应 Java 构造器 `IndexInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        IndexInstruction { error_reporter }
    }

    fn nonindexable(&self, index_able: &DataValue) -> QLException {
        self.error_reporter.report_format(
            error_codes::NONINDEXABLE_OBJECT,
            error_codes::error_msg(error_codes::NONINDEXABLE_OBJECT),
            &[if index_able.is_null() {
                "null".to_string()
            } else {
                index_able.data_type_name().to_string()
            }],
        )
    }
}

impl QLInstruction for IndexInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let index = q_context.pop().get();
        let index_able = q_context.pop().get();
        match &index_able {
            DataValue::List(list) => {
                let index_number = assert_number(
                    &index,
                    error_codes::INVALID_INDEX,
                    error_codes::error_msg(error_codes::INVALID_INDEX),
                    &*self.error_reporter,
                )?;
                let len = list.borrow().len() as i64;
                let int_index = java_index(len, index_number);
                if int_index < 0 || int_index >= len {
                    return Err(self.error_reporter.report(
                        error_codes::INDEX_OUT_BOUND,
                        error_codes::error_msg(error_codes::INDEX_OUT_BOUND),
                    ));
                }
                q_context.push(QValue::Left(Rc::new(std::cell::RefCell::new(
                    ListItemValue::new(Rc::clone(list), int_index as usize),
                ))));
            }
            DataValue::Map(map) => {
                q_context.push(QValue::Left(Rc::new(std::cell::RefCell::new(
                    MapItemValue::new(Rc::clone(map), index),
                ))));
            }
            DataValue::Array(arr) => {
                let index_number = assert_number(
                    &index,
                    error_codes::INVALID_INDEX,
                    error_codes::error_msg(error_codes::INVALID_INDEX),
                    &*self.error_reporter,
                )?;
                let len = arr.borrow().len() as i64;
                let int_index = java_index(len, index_number);
                if int_index < 0 || int_index >= len {
                    return Err(self.error_reporter.report(
                        error_codes::INDEX_OUT_BOUND,
                        error_codes::error_msg(error_codes::INDEX_OUT_BOUND),
                    ));
                }
                q_context.push(QValue::Left(Rc::new(std::cell::RefCell::new(
                    ArrayItemValue::new(Rc::clone(arr), int_index as usize),
                ))));
            }
            DataValue::Null if ql_options.is_avoid_null_pointer() => {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
            }
            _ => return Err(self.nonindexable(&index_able)),
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        2
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Index"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

