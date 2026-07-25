use super::position::Position;

/// A range in a document, mirroring Java `lsp.Range`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Range {
    start: Position,
    end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Range { start, end }
    }

    pub fn start(&self) -> Position {
        self.start
    }

    pub fn end(&self) -> Position {
        self.end
    }
}
