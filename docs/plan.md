# QlExpress Rust 迁移计划（历史入口）

本文件保留为早期 Stage 0–7 计划的兼容入口。迁移已经完成，当前事实来源是：

1. [迁移路线图](迁移路线图.md)：阶段状态、架构决策和质量门禁；
2. [对象级对照表](对象级对照表.md)：237 个 Java 对象逐项映射；
3. [语义迁移对照表](语义迁移对照表.md)：运行时功能与 Java→Rust 技术映射；
4. [对象名称一致性检查](对象名称一致性检查.md)：文件、包路径与一文件一对象验收。

## 当前状态（2026-07-29）

| 项目 | 结果 |
|---|---|
| Java 基线 | QLExpress 4.2.0-beta，237 个生产对象 |
| Rust 生产对象文件 | 251（core 247 + derive 4） |
| 对象职责覆盖 | 237/237 |
| 合并待拆 / 缺失 / stub | 0 / 0 / 0 |
| 测试 | 803 个 Rust `#[test]` 静态清单；全工作区实跑通过 / 0 failed / 0 ignored |
| Java 测试 | Maven 225 passed；SOURCE_PARITY 223 个注解方法逐项登记 |
| 核心行覆盖率 | cargo-llvm-cov 20,217 / 23,787 = 84.99%，高于 Java 84.84% |
| 格式 | `cargo fmt --all -- --check` 通过 |
| 静态检查 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过 |

## 已完成阶段

```mermaid
flowchart LR
    S0["S0 对照基线"] --> S1["S1 工程与基础对象"]
    S1 --> S2["S2 Lexer / Parser / Visitor"]
    S2 --> S3["S3 指令编译与 QVM"]
    S3 --> S4["S4 操作符与数值"]
    S4 --> S5["S5 函数 / 上下文 / 安全 / Runner"]
    S5 --> S6["S6 Java 对齐测试"]
    S6 --> S7["S7 对象拆分、trace、文档与严格门禁"]
```

关键收尾包括：

- `TraceExpressionVisitor` 完整遍历 AST，并与运行时 `ExpressionTrace`、
  `TracePointTree`、`QTraces` 形成闭环；
- `BigInteger` 使用 `num_bigint::BigInt`，不再受 `i128` 固定宽度限制；
- Java `FixedSizeStack` 容量语义进入真实 QVM 作用域和 lambda 执行路径；
- 构造器、内建兼容成员、宿主动态字段/方法统一执行安全策略；
- `DataValue`、`Value`、`QValue` 和
  `runtime.data.lambda.QLambdaMethod` 已拆分到独立职责文件；
- 删除历史聚合指令文件，生产源码无 `compat.rs`、`todo!`、
  `unimplemented!` 或忽略测试。

## 生产验收边界

仓库级源码、对象映射、静态门禁和测试已完成，详细证据见
[技术要求](QLExpress-Rust-技术要求.md) 与
[迁移测试对照表](迁移测试对照表.md)。具体业务宿主仍需验证其
`NativeRegistry` 注册内容、真实数据、容量目标、部署拓扑、监控告警与回滚流程；
这些环境事实不由本迁移仓库的单元/集成测试代替。
