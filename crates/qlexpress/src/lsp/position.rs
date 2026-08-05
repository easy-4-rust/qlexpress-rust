/// LSP 使用的零基行列位置。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/lsp/Position.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Zero-based position in a document, mirroring Java `lsp.Position`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.exception.lsp.Position。
pub struct Position {
    /// Line position in a document (zero-based).
    line: i32,
    /// Character offset on a line (zero-based).
    character: i32,
}

impl Position {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Position.java:20` 的 `Position::<init>`。
    pub fn new(line: i32, character: i32) -> Self {
        Position { line, character }
    }

    /// 返回 LSP 零基行号。
    /// 对应 Java: `Position#line`。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 返回 LSP 零基字符列。
    /// 对应 Java: `Position#character`。
    pub fn character(&self) -> i32 {
        self.character
    }
}

#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn constructor_and_accessors_preserve_raw_java_int_values() {
        let position = Position::new(7, 99);
        assert_eq!(position.line(), 7);
        assert_eq!(position.character(), 99);

        // Java 构造器只保存字段，不会替调用方截断超长列或拒绝负数。
        let negative = Position::new(-1, -2);
        assert_eq!(negative.line(), -1);
        assert_eq!(negative.character(), -2);
    }
}
