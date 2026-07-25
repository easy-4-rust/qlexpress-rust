//! 空 Lambda 定义,对应 Java `com.alibaba.qlexpress4.runtime.QLambdaDefinitionEmpty`。
//! 职责:空 Lambda 的编译期形态(单例),物化后为空 Lambda。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;

use crate::ql_options::QLOptions;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_empty::QLambdaEmpty;
use crate::utils::println_utils::PrintlnUtils;

/// 空 Lambda 定义。对应 Java: com.alibaba.qlexpress4.runtime.QLambdaDefinitionEmpty
/// Java `QLambdaDefinitionEmpty.INSTANCE`.
pub struct QLambdaDefinitionEmpty;

impl QLambdaDefinitionEmpty {
    /// 单例。对应 Java `QLambdaDefinitionEmpty.INSTANCE`。
    /// Java `QLambdaDefinitionEmpty.INSTANCE`.
    pub const INSTANCE: QLambdaDefinitionEmpty = QLambdaDefinitionEmpty;
}

impl QLambdaDefinition for QLambdaDefinitionEmpty {
    /// 向下转型支持(供 api/parsecache Exporter 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    /// 物化为空 Lambda。对应 Java 方法 `toLambda`(返回 `QLambdaEmpty.INSTANCE`)。
    fn to_lambda(
        self: Rc<Self>,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        _new_env: bool,
    ) -> Rc<QLambda> {
        Rc::new(QLambda::Empty(QLambdaEmpty))
    }

    fn println(&self, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, self.name(), debug);
    }

    fn name(&self) -> &str {
        "EmptyLambdaDefinition"
    }
}
