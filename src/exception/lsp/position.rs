/// Zero-based position in a document, mirroring Java `lsp.Position`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Position {
    /// Line position in a document (zero-based).
    line: i32,
    /// Character offset on a line (zero-based).
    character: i32,
}

impl Position {
    pub fn new(line: i32, character: i32) -> Self {
        Position { line, character }
    }

    pub fn line(&self) -> i32 {
        self.line
    }

    pub fn character(&self) -> i32 {
        self.character
    }
}
