//! 切片指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.SliceInstruction`。
//! 职责:数组/列表切片。
//! 本文件由 `index.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::util::value_utils::{assert_number, java_index};
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

pub use super::slice_mode::SliceMode;

/// 切片指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.SliceInstruction(职责:数组/列表切片)
/// 操作：切片数组或列表，例如 `a[2:4]`、`a[4:-1]`、`a[:4]`、`a[5:]`、`a[:]`。
/// Input: 0-2
/// Output: 1
///
/// Mirrors Java `SliceInstruction`.
pub struct SliceInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    mode: SliceMode,
}

impl SliceInstruction {
    /// 构造指令,对应 Java 构造器 `SliceInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, mode: SliceMode) -> Self {
        SliceInstruction {
            error_reporter,
            mode,
        }
    }

    /// 对应 Java 方法 `mode`。
    pub fn mode(&self) -> SliceMode {
        self.mode
    }

    fn assert_index(&self, value: &DataValue) -> Result<i64, QLException> {
        assert_number(
            value,
            error_codes::INVALID_INDEX,
            error_codes::error_msg(error_codes::INVALID_INDEX),
            &*self.error_reporter,
        )
    }

    /// Java `indexAbleLen`.
    fn index_able_len(&self, index_able: &DataValue) -> Result<i64, QLException> {
        match index_able {
            DataValue::List(l) => Ok(l.borrow().len() as i64),
            DataValue::Array(a) => Ok(a.borrow().len() as i64),
            _ => Err(self.error_reporter.report_format(
                error_codes::NONINDEXABLE_OBJECT,
                error_codes::error_msg(error_codes::NONINDEXABLE_OBJECT),
                &[if index_able.is_null() {
                    "null".to_string()
                } else {
                    index_able.data_type_name().to_string()
                }],
            )),
        }
    }
}

impl QLInstruction for SliceInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut start_int: i64 = 0;
        let end_int: i64;
        let index_able: DataValue;
        match self.mode {
            SliceMode::Both => {
                let end = q_context.pop().get();
                let start = q_context.pop().get();
                index_able = q_context.pop().get();
                start_int = self.assert_index(&start)?;
                end_int = self.assert_index(&end)?;
            }
            SliceMode::Left => {
                let end = q_context.pop().get();
                index_able = q_context.pop().get();
                end_int = self.assert_index(&end)?;
            }
            SliceMode::Right => {
                let start = q_context.pop().get();
                index_able = q_context.pop().get();
                start_int = self.assert_index(&start)?;
                end_int = self.index_able_len(&index_able)?;
            }
            SliceMode::Copy => {
                index_able = q_context.pop().get();
                end_int = self.index_able_len(&index_able)?;
            }
        }

        match &index_able {
            DataValue::List(list) => {
                // Java listSlice
                let len = list.borrow().len() as i64;
                let start = java_index(len, start_int).max(0);
                let end = java_index(len, end_int).min(len);
                let result = if start >= end {
                    DataValue::list(vec![])
                } else {
                    DataValue::list(list.borrow()[start as usize..end as usize].to_vec())
                };
                q_context.push(QValue::Data(result));
            }
            DataValue::Array(arr) => {
                // Java arraySlice
                let borrowed = arr.borrow();
                let len = borrowed.len() as i64;
                let start = java_index(len, start_int).max(0);
                let end = java_index(len, end_int).min(len);
                let values = if start >= end {
                    vec![]
                } else {
                    borrowed[start as usize..end as usize].to_vec()
                };
                let result = DataValue::Array(Rc::new(std::cell::RefCell::new(
                    borrowed.copy_with_values(values),
                )));
                q_context.push(QValue::Data(result));
            }
            DataValue::Null if ql_options.is_avoid_null_pointer() => {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
            }
            _ => {
                return Err(self.error_reporter.report_format(
                    error_codes::NONINDEXABLE_OBJECT,
                    error_codes::error_msg(error_codes::NONINDEXABLE_OBJECT),
                    &[if index_able.is_null() {
                        "null".to_string()
                    } else {
                        index_able.data_type_name().to_string()
                    }],
                ));
            }
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        match self.mode {
            SliceMode::Both => 2,
            SliceMode::Copy => 0,
            _ => 1,
        }
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Slice"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
