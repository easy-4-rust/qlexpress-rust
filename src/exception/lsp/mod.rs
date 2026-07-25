//! LSP-style error location model, mirroring Java `exception/lsp/`.

pub mod diagnostic;
pub mod position;
pub mod range;

pub use diagnostic::Diagnostic;
pub use position::Position;
pub use range::Range;
