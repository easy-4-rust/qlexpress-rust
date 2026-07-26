use super::position::Position;

/// `Range` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/lsp/Range.java`；具体对象路径见 `docs/对象级对照表.md`。
/// A range in a document, mirroring Java `lsp.Range`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Range {
    start: Position,
    end: Position,
}

impl Range {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Range.java:12` 的 `Range::<init>`。
    pub fn new(start: Position, end: Position) -> Self {
        Range { start, end }
    }

    /// 返回诊断范围的起始位置。
    /// 对应 Java: `Range#start`。
    pub fn start(&self) -> Position {
        self.start
    }

    /// 返回诊断范围的结束位置。
    /// 对应 Java: `Range#end`。
    pub fn end(&self) -> Position {
        self.end
    }
}
