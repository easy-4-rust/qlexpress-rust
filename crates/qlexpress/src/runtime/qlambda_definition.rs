//! Lambda 定义(编译期)契约,对应 Java `com.alibaba.qlexpress4.runtime.QLambdaDefinition`。
//! 职责:描述一个 Lambda 的编译期形态,并在运行时物化为可调用的 `QLambda`。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;

use crate::ql_options::QLOptions;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;

/// Lambda 定义(编译期)。对应 Java: com.alibaba.qlexpress4.runtime.QLambdaDefinition
///
/// A lambda definition (compile-time), mirroring Java `QLambdaDefinition`.
pub trait QLambdaDefinition {
    /// 物化为可调用 Lambda。对应 Java 方法 `toLambda(QContext, QLOptions, boolean newEnv)`。
    /// Java `toLambda(QContext, QLOptions, boolean newEnv)`.
    fn to_lambda(
        self: Rc<Self>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        new_env: bool,
    ) -> Rc<QLambda>;

    /// 调试打印。对应 Java 方法 `println(int depth, Consumer<String> debug)`。
    /// Java `println(int depth, Consumer<String> debug)`.
    fn println(&self, depth: usize, debug: &mut dyn FnMut(String));

    /// 获取名字。对应 Java 方法 `getName()`。
    /// Java `getName()`.
    fn name(&self) -> &str;

    /// 向下转型支持(Java `instanceof QLambdaDefinitionInner` 的 Rust 等价物),
    /// 供 `api/parsecache` Exporter 分派导出。默认 `None`。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}
