# Changelog

本项目遵循语义化版本，并将预发布版本用于兼容性与生产验收。

## [0.1.0-alpha.1] - 2026-07-27

首个公开 alpha 版本，对齐 QLExpress Java `4.2.0-beta`
（commit `9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3`）。

### Added

- 表达式解析、字节码编译与 QVM 执行能力。
- Java/Rust 自动差分、真实脚本回放、并发、负载、稳定性与安全 fuzz 验收框架。
- `QLExpressType` derive 宏，以及从 `qlexpress` 主 crate 的统一重导出。

### Known limitations

- 这是 API 仍可能调整的 alpha 版本，不承诺跨 alpha 的稳定兼容性。
- 生产采用仍需由业务宿主完成灰度、监控和回滚验证。

[0.1.0-alpha.1]: https://github.com/easy-4-rust/qlexpress-rust/releases/tag/v0.1.0-alpha.1
