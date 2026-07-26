//! 操作符基类包,对应 Java `com.alibaba.qlexpress4.runtime.operator.base`。
//!
//! Stage 4a 整理( SPEC §5.5 一类一文件 ):
//! - `BinaryOperator` trait 迁至 operator 顶层 `binary_operator.rs`
//!   (Java BinaryOperator.java 在顶层包),此处 re-export 保持
//!   `base::BinaryOperator` 既有引用可用;
//! - `UnaryOperator` trait 暂留本文件 —— 它对应 Java
//!   `operator/unary/UnaryOperator.java`(Stage 4b 另一 agent 负责),
//!   为不破坏 `base::UnaryOperator` 既有引用暂不迁移,待 Stage 4b 归位;
//! - 新增 `base_binary_operator.rs` / `base_unary_operator.rs`
//!   (Java 同名抽象基类的公共计算逻辑)。

pub mod base_binary_operator;
pub mod base_unary_operator;

pub use super::binary_operator::BinaryOperator;
pub use base_binary_operator::BaseBinaryOperator;
pub use base_unary_operator::BaseUnaryOperator;

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::value::{DataValue, QValue};

/// 一元操作符(前缀 `++ -- ! ~ + -`、后缀 `++ --`)。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.UnaryOperator
/// (暂存于 base 模块,见模块头注释;Stage 4b 迁往 unary/ 目录)。
pub trait UnaryOperator {
    /// 执行一元运算。
    ///
    /// 对应 Java 方法: `execute(Value value, ErrorReporter errorReporter)`,
    /// 返回 Java `Object`。
    fn execute(
        &self,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException>;

    /// 对应 Java 方法: `Operator.getOperator()` —— 操作符词素(如 `"!"`)。
    fn operator(&self) -> &str;

    /// 对应 Java 方法: `Operator.getPriority()` —— 优先级;
    /// Java 实现里一元操作符报 QLPrecedences.UNARY 级别,
    /// 该值仅二元操作符会被查询。
    fn priority(&self) -> i32;
}
