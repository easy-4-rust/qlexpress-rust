//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 LambdaParametersContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LambdaParametersContext
/// Java `LambdaParametersContext`: single id (`x -> ..`) or parameter list.
#[derive(Clone, Debug)]
pub struct LambdaParametersContext {
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Option<Box<Node>>,
    /// 该语法规则中的 `params` 子节点、终结符或节点集合。
    pub params: Option<Box<Node>>,
}
