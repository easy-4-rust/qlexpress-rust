//! 映射字面量指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.NewMapInstruction`。
//! 职责:以键值对创建 Map。
//! 本文件由 `new_instance.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// 映射字面量指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.NewMapInstruction(职责:以键值对创建 Map)
/// Operation: new a Map with top ${keys.length} stack element
/// Input: ${keys.length}
/// Output: 1
///
/// Mirrors Java `NewMapInstruction`.
pub struct NewMapInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    keys: Vec<String>,
}

impl NewMapInstruction {
    /// 构造指令,对应 Java 构造器 `NewMapInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, keys: Vec<String>) -> Self {
        NewMapInstruction {
            error_reporter,
            keys,
        }
    }

    /// 对应 Java 方法 `keys`。
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

impl QLInstruction for NewMapInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let init_items = q_context.pop_n(self.keys.len());
        let mut map = IndexMap::new();
        for (i, key) in self.keys.iter().enumerate() {
            map.insert(DataValue::Str(key.clone()), init_items.get_value(i));
        }
        q_context.push(QValue::Data(DataValue::map(map)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.keys.len() as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: NewMap by keys [{}]", index, self.keys.join(", ")),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

