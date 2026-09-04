# Changelog

本项目遵循语义化版本，并将预发布版本用于兼容性与生产验收。

## [Unreleased]

### Deprecated

- **`Express4Runner::execute_with_alias_values`**：已标记 `#[deprecated]`，替代方法为
  `execute_with_alias_objects`（保持与 Java `executeWithAliasObjects` 的名称一致性）。
  计划在 1.0 移除。

### API Stability

- 建立 [API 稳定性承诺文档](docs/API-Stability.md)，涵盖：
  - 公共 API 表面枚举（`lib.rs` 全部 re-export + 各模块主要公共类型）
  - SemVer 策略与稳定性分类（stable / unstable）
  - 1.0 之前可能变动的项清单
  - 从 beta.0 到 1.0 的迁移指南
  - API 问题报告指引

## [0.1.0-beta.0] - 2026-09-03

> 本版本相对 0.1.0-alpha.2：**可观测性 + 宿主协作式截止时间契约 +
> 对齐证据台账 + 验收矩阵当日全绿**，通道由 alpha 晋级 beta。
> 逐项证据见 [docs/生产验收.md](docs/生产验收.md) 2026-09-03 轮。

### Added

- **tracing 可观测性**（feature gate `tracing`，默认关闭零开销）：
  parse/compile/execute 边界 span 与事件；`coverage.yml` 覆盖率工作流
- **宿主协作式截止时间契约**：宿主函数可经 `QContext::is_expired()`
  主动检测过期；返回的 `SANDBOX_DEADLINE_EXCEEDED`（Timeout）原样传播，
  不被 `INVOKE_FUNCTION_INNER_ERROR` 归一化掩码；其余宿主错误仍包裹
- **对齐证据体系**：alignment 标记/指令/操作符/运行时证据测试 8 文件、
  `host_deadline_contract` 契约测试、perf fixtures（复杂数据处理/长行脚本）、
  `migration-dispositions.json` 台账（237 对象 + 1,814 方法逐项处置）

### Changed

- 性能冒烟阈值 200ms→1000ms（fib/列表迭代）：CI 共享 runner debug
  构建波动所致，release 实测 <50ms，仍拦截量级回归
- libFuzzer 加 `-detect_leaks=0`：LSan 对退出时静态持有 ~2KB 分配误报；
  fuzz 验收目标保持 crash/hang/OOM
- Windows 平台声明：受限进程契约（os_limits）为 Unix 专属，Windows 由
  Job Object 等外部沙箱承担硬限制，worker 返回 `WORKER_ERROR`

### Fixed

- Linux-gnu `libc::setrlimit/getrlimit` 首参类型（u32）与 macOS（i32）
  差异导致 Linux 编译失败（6 处 E0308）
- Windows checkout autocrlf 破坏不可变语料逐字节 SHA → `.gitattributes`
  对 `qlexpress-test/tests/suite/source/**` 关闭文本变换
- `*.json` 全局 ignore 吞掉 `docs/source-test-parity.json` 清单 → 反白入库
- `dtolnay/rust-toolchain`（钉 SHA）将 `toolchain` 变必填后 5 处工作流
  步骤失败（Production Readiness 自 08-08 红灯）→ 全部补齐

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
