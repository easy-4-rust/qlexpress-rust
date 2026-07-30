//! LSP-style error location model, mirroring Java `exception/lsp/`.

/// `diagnostic` 子模块。
pub mod diagnostic;
/// `position` 子模块。
pub mod position;
/// `range` 子模块。
pub mod range;

pub use diagnostic::Diagnostic;
pub use position::Position;
pub use range::Range;
