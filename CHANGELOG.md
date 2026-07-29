# Changelog

本项目遵循语义化版本，并将预发布版本用于兼容性与生产验收。

## [0.1.0-alpha.2] - 2026-07-30

第二个公开 alpha 版本，继续对齐固定的 QLExpress Java `4.2.0-beta`
（commit `9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3`）。

### Fixed

- 补齐分类对象字面量对原生对象字段的写入，并保持 `readonly` 拒写及类型转换语义。
- 对齐 List/Map 的 Java `toString()` 形态。
- 修复 `Integer/Long MIN_VALUE / -1` 与 `% -1` 被误判为除零的问题。
- 避免在 LLVM 覆盖率插桩构建中执行失真的墙钟性能阈值；普通测试和独立负载门禁保持严格。

### Added

- 增加懒参数、调用指令、表达式追踪、parse cache、成员调用和数值边界矩阵。
- 建立 223 行 SOURCE_PARITY 及 RUST_OBLIGATION、VALUE_ADD 三类迁移测试台账。
- 增加迁移技术要求、生产验收与对象统计校正文档。
- 核心生产代码行覆盖率达到 84.99%，高于固定 Java 基线 84.84%。

### Verification

- Rust workspace 803 个测试函数全部通过，0 failed / 0 ignored。
- Java/Rust 差分 50/50，真实 Java 资源脚本回放 151/151。
- 16,000 次并发、25,000 次确定性安全 fuzz、76,997 次 libFuzzer 均无错误。
- 15 秒负载 152,107 次执行、0 错误、p99 4.859ms。

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

[0.1.0-alpha.2]: https://github.com/easy-4-rust/qlexpress-rust/releases/tag/v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/easy-4-rust/qlexpress-rust/releases/tag/v0.1.0-alpha.1
