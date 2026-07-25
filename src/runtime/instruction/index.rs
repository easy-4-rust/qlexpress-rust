//! Index/slice instructions, mirroring Java `IndexInstruction` and
//! `SliceInstruction`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::data::{ArrayItemValue, ListItemValue, MapItemValue};
use crate::runtime::instruction::QLInstruction;
use crate::runtime::qcontext::QContext;
use crate::runtime::util::value_utils::{assert_number, java_index};
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Operation: extract value with index, like a[0], m['a']
/// Input: 2, indexable object and index
/// Output: 1
///
/// Mirrors Java `IndexInstruction`.
pub struct IndexInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl IndexInstruction {
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

/// Java `SliceInstruction.Mode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceMode {
    Left,
    Right,
    Both,
    Copy,
}

/// Operation: slice array or list, like a[2:4], a[4:-1], a[:4], a[5:], a[:]
/// Input: 0-2
/// Output: 1
///
/// Mirrors Java `SliceInstruction`.
pub struct SliceInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    mode: SliceMode,
}

impl SliceInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, mode: SliceMode) -> Self {
        SliceInstruction {
            error_reporter,
            mode,
        }
    }

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
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut start_int: i64 = 0;
        let mut end_int: i64;
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
                let len = arr.borrow().len() as i64;
                let start = java_index(len, start_int).max(0);
                let end = java_index(len, end_int).min(len);
                let result = if start >= end {
                    DataValue::array(vec![])
                } else {
                    DataValue::array(arr.borrow()[start as usize..end as usize].to_vec())
                };
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
                ))
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
