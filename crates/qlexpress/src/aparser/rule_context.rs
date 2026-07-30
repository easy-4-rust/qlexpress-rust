//! 规则上下文的孩子访问体系,对应 Java `com.alibaba.qlexpress4.aparser.RuleContext`。
//! 职责:提供语法节点的有序孩子枚举(Java `RuleContext.children`)、
//! start/stop 边界计算。Java 无独立 ChildRef/HasChildren 类,此处为 Rust 适配
//! (Java 以 `List<ParseTree> children` 存储,Rust 以借用枚举 + trait 实现同等语义)。
//! 本文件由 `syntax_tree.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

pub use super::child_ref::ChildRef;
use super::syntax_tree_factory::Node;
use super::terminal_node::TerminalNode;
use super::token::Token;

impl<'a> ChildRef<'a> {
    /// 返回当前节点或 Token 对应的源码文本。
    /// 无显式参数；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ParseTree.java`，方法 `text`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `ParseTree.getText`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#text。
    pub fn text(&self) -> String {
        match self {
            ChildRef::Node(n) => n.text(),
            ChildRef::Term(t) => t.text().to_string(),
        }
    }

    /// 返回当前语法节点的起始 Token。
    /// 无显式参数；返回：`Option<&'a Token>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/RuleContext.java`，方法 `startToken`；Rust 侧按所有权与 `Result` 语义适配。
    /// First token covered by this child (Java bounds computation).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#startToken。
    pub fn start_token(&self) -> Option<&'a Token> {
        match self {
            ChildRef::Node(n) => n.start_token(),
            ChildRef::Term(t) => Some(t.symbol()),
        }
    }

    /// 返回当前语法节点的结束 Token。
    /// 无显式参数；返回：`Option<&'a Token>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/RuleContext.java`，方法 `stopToken`；Rust 侧按所有权与 `Result` 语义适配。
    /// Last token covered by this child.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#stopToken。
    pub fn stop_token(&self) -> Option<&'a Token> {
        match self {
            ChildRef::Node(n) => n.stop_token(),
            ChildRef::Term(t) => Some(t.symbol()),
        }
    }
}

/// 可按源码顺序枚举孩子的节点契约。对应 Java: `RuleContext.children` 访问能力(Rust 适配 trait)
/// Anything that can enumerate its children in source order.
pub trait RuleContext {
    /// 处理 children 对应的接口职责。
    /// 无显式参数；返回：`Vec<ChildRef<'_>>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/RuleContext.java`，方法 `children`。
    /// Children in the exact order the Java parser `addChild`ed them.
    fn children(&self) -> Vec<ChildRef<'_>>;
}

/// 兼容既有 Rust API 的名称；实际 Java 对偶对象为 [`RuleContext`]。
pub use RuleContext as HasChildren;

// ---------------------------------------------------------------------------
// Helper constructors used by the `children()` implementations.
// ---------------------------------------------------------------------------

/// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#n。
pub(crate) fn n(node: &Node) -> ChildRef<'_> {
    ChildRef::Node(node)
}

/// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#t。
pub(crate) fn t(term: &TerminalNode) -> ChildRef<'_> {
    ChildRef::Term(term)
}

/// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#pushOpt。
pub(crate) fn push_opt<'a>(out: &mut Vec<ChildRef<'a>>, opt: &'a Option<Box<Node>>) {
    if let Some(node) = opt {
        out.push(n(node));
    }
}

/// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#pushOptTerm。
pub(crate) fn push_opt_term<'a>(out: &mut Vec<ChildRef<'a>>, opt: &'a Option<TerminalNode>) {
    if let Some(term) = opt {
        out.push(t(term));
    }
}

/// 对应 Java: com.alibaba.qlexpress4.aparser.RuleContext#pushAll。
pub(crate) fn push_all<'a>(out: &mut Vec<ChildRef<'a>>, list: &'a [Node]) {
    for node in list {
        out.push(n(node));
    }
}

/// 按 `节点, 分隔符, 节点...` 的 Java `children` 顺序写入列表。
///
/// `separators` 可以与 `nodes` 等长，用于保留 Java 允许的尾随逗号；
/// 也可以比 `nodes` 少一个，用于普通分隔列表。
pub(crate) fn push_interleaved<'a>(
    out: &mut Vec<ChildRef<'a>>,
    nodes: &'a [Node],
    separators: &'a [TerminalNode],
) {
    for (index, node) in nodes.iter().enumerate() {
        out.push(n(node));
        if let Some(separator) = separators.get(index) {
            out.push(t(separator));
        }
    }
}
