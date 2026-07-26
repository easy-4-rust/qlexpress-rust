use super::position::Position;

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

    /// 执行 `start` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Range.java:1` 的 `Range`；该方法为 Rust 同职责适配接口。
    pub fn start(&self) -> Position {
        self.start
    }

    /// 执行 `end` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Range.java:1` 的 `Range`；该方法为 Rust 同职责适配接口。
    pub fn end(&self) -> Position {
        self.end
    }
}
