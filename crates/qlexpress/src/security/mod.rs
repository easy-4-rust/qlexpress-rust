//! Security model mirroring Java `com.alibaba.qlexpress4.security`.
//!
//! - [`ql_security_strategy`]:外观枚举 + `NativeMember` 成员描述符
//!   (Java `QLSecurityStrategy` 接口及其静态工厂);
//! - 四个策略文件一一对应 Java 策略类(StrategyOpen / StrategyIsolation /
//!   StrategyBlackList / StrategyWhiteList)。
//!
//! 成员分派的实际接线点(指令执行时把解析出的 `NativeMember` 交给策略
//! `check`)在 Express4Runner 阶段完成。

pub mod ql_security_strategy;
pub mod strategy_black_list;
pub mod strategy_isolation;
pub mod strategy_open;
pub mod strategy_white_list;

pub use ql_security_strategy::{NativeMember, QLSecurityStrategy};
pub use strategy_black_list::StrategyBlackList;
pub use strategy_isolation::StrategyIsolation;
pub use strategy_open::StrategyOpen;
pub use strategy_white_list::StrategyWhiteList;
