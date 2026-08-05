use super::position::Position;

/// 由起止位置描述的 LSP 源码范围。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/lsp/Range.java`；具体对象路径见 `docs/对象级对照表.md`。
/// A range in a document, mirroring Java `lsp.Range`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.exception.lsp.Range。
pub struct Range {
    start: Option<Position>,
    end: Option<Position>,
}

impl Range {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Range.java:12` 的 `Range::<init>`。
    pub fn new(start: Position, end: Position) -> Self {
        Range {
            start: Some(start),
            end: Some(end),
        }
    }

    /// 以可空端点构造范围。
    ///
    /// 对应 Java: `Range(Position start, Position end)` 可接收并原样保存
    /// `null`。普通非空范围优先使用 [`Self::new`]。
    pub fn from_options(start: Option<Position>, end: Option<Position>) -> Self {
        Range { start, end }
    }

    /// 返回诊断范围的起始位置。
    /// 对应 Java: `Range#start`。
    pub fn start(&self) -> Option<&Position> {
        self.start.as_ref()
    }

    /// 返回诊断范围的结束位置。
    /// 对应 Java: `Range#end`。
    pub fn end(&self) -> Option<&Position> {
        self.end.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, Range};

    #[test]
    fn preserves_non_null_and_null_java_endpoints() {
        let range = Range::new(Position::new(1, 2), Position::new(3, 4));
        assert_eq!(range.start().map(Position::line), Some(1));
        assert_eq!(range.start().map(Position::character), Some(2));
        assert_eq!(range.end().map(Position::line), Some(3));
        assert_eq!(range.end().map(Position::character), Some(4));

        let nullable = Range::from_options(None, None);
        assert!(nullable.start().is_none());
        assert!(nullable.end().is_none());
        assert_eq!(Range::default(), nullable);
    }
}
