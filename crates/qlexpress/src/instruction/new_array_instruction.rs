//! 新建数组指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.NewArrayInstruction`。
//! 职责:创建指定长度数组。
//! 本文件由 `new_instance.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::data::convert::obj_type_convertor::{ObjTypeConvertor, TargetType};
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 新建数组指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.NewArrayInstruction(职责:创建指定长度数组)
/// new int[] {1,2,3}
/// Operation: new array with init items
/// Input: ${length}
/// Output: 1
///
/// Mirrors Java `NewArrayInstruction`.
pub struct NewArrayInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    clz: TargetType,
    length: usize,
}

impl NewArrayInstruction {
    /// 构造指令,对应 Java 构造器 `NewArrayInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, clz: TargetType, length: usize) -> Self {
        NewArrayInstruction {
            error_reporter,
            clz,
            length,
        }
    }

    /// 对应 Java 方法 `clz`。
    pub fn clz(&self) -> TargetType {
        self.clz
    }

    /// 对应 Java 方法 `length`。
    pub fn length(&self) -> usize {
        self.length
    }
}

impl QLInstruction for NewArrayInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        if let Some(budget) = q_context.q_runtime().execution_budget() {
            budget.charge_collection_items(self.length)?;
        }
        if !ql_options.check_arr_len(self.length as i32) {
            return Err(self.error_reporter.report_format(
                error_codes::EXCEED_MAX_ARR_LENGTH,
                error_codes::error_msg(error_codes::EXCEED_MAX_ARR_LENGTH),
                &[
                    self.length.to_string(),
                    ql_options.max_arr_length().to_string(),
                ],
            ));
        }
        let init_items = q_context.pop_n(self.length);
        let mut array = Vec::with_capacity(self.length);
        for i in 0..init_items.size() {
            let init_item_obj = init_items.get_value(i);
            let ql_convert_result = ObjTypeConvertor::cast(&init_item_obj, self.clz);
            if !ql_convert_result.is_convertible() {
                return Err(self.error_reporter.report_format(
                    error_codes::INCOMPATIBLE_ARRAY_ITEM_TYPE,
                    error_codes::error_msg(error_codes::INCOMPATIBLE_ARRAY_ITEM_TYPE),
                    &[
                        i.to_string(),
                        if init_item_obj.is_null() {
                            "null".to_string()
                        } else {
                            init_item_obj.data_type_name().to_string()
                        },
                        self.clz.java_name().to_string(),
                    ],
                ));
            }
            array.push(ql_convert_result.into_converted());
        }
        q_context.push(QValue::Data(DataValue::array(array)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.length as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: NewArray with length {}", index, self.length),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
