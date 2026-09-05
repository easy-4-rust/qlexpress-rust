# 业务验收

本目录记录 qlexpress-rust **从工程就绪跨到生产就绪**的实测证据。仅依赖仓库自身资源（不要求用户提供业务脚本）。

## 已完成的验收面

### Q1: 错误码分布（仓库级基线）

- 脚本：[scripts/analyze_error_distribution.py](/Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust/scripts/analyze_error_distribution.py)
- 报告：[error-distribution.md](/Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust/docs/业务验收/error-distribution.md)
- 关键结论：295 差分用例覆盖 9/66 错误码；57 个未覆盖（主要集中在 function invocation、control flow、type cast、try/catch 域）；**后续业务脚本验收应优先针对这 57 个未覆盖错误码构造边界用例**

### Q3: 多进程隔离

- Harness: `cargo run --release -p qlexpress-verification -- multi-process-isolation 8 30`
- 三个场景全部 PASS：
  - **一致性**：8 进程并发跑同一脚本 200 次，checksum 全相等（无状态污染）
  - **RLIMIT_NPROC**：fork-bomb 风格脚本被 RLIMIT 拦截（实测 45 个 sleep 子进程后被阻）
  - **panic 隔离**：1 个 panic 进程非零退出，其他 7 个正常返回（无交叉污染）
- 实现: [crates/qlexpress-verification/src/multi_process_isolation.rs](/Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust/crates/qlexpress-verification/src/multi_process_isolation.rs)

### Q4: 性能基线（30 秒 × 8 线程，dev 分支）

- 实测数据：
  - **52,562 ops/s**（中位数）
  - **p99 = 874 µs**
  - 0 errors
- 与历史对比：
  - 08-08（旧版）：8,596 ops/s、p99 5.08ms
  - 09-03：55,135 ops/s、p99 887µs
  - **本轮与 09-03 高度一致**（吞吐 -4.7%、p99 -1.5%），P1-2 regex 缓存 + P2-1 StringJoin 优化未导致性能回退
- 业务启示：99% 请求 < 1ms；是常见业务负载要求（100 req/s）的 **525 倍**余量
- 报告: [perf-baseline.md](/Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust/docs/业务验收/perf-baseline.md)

## 仍未完成的验收面（依赖用户提供业务脚本）

### Q2: 调用栈深度

- 状态：**可跳过**——`max_stack_size` 编译期计算、268 个 try_push/try_pop 测试覆盖正常路径；需要业务脚本的"最深深度"作为样本，仓库无此样本
- 建议：业务脚本出现"接近预算"的场景时再回头跑

### Q1-biz: 业务脚本真实错误码分布

- 仓库级 Q1 给出 295 差分基线，但**业务真实错误谱**未知
- 建议：用户提供 100-1000 条脱敏业务脚本到 `scripts/business-corpus/`，跑 `scripts/analyze_error_distribution.py --corpus scripts/business-corpus/`，对比仓库基线

## 总体判断

| 面 | 状态 | 关键证据 |
|---|---|---|
| 工程代码 | ✅ | 1453+ 测试全绿、4 门禁 0 警告、11 项审查风险闭环、UNVERIFIED 归零 |
| 仓库级错误码 | ✅ | Q1 报告 + 57 未覆盖错误码清单 |
| 多进程隔离 | ✅ | Q3 三个场景全 PASS |
| 性能 | ✅ | Q4 52,562 ops/s、p99 < 1ms |
| 业务错误码真实分布 | ⚠️ | 待用户提供业务脚本 |
| 业务调用栈深度 | ⚠️ | 待业务场景出现时再跑 |

**结论**：仓库级 + 隔离 + 性能三项**已具备生产部署条件**。业务脚本级验收（Q1-biz / Q2）作为后续运营动作，**不阻断发布到 0.1.0-beta.1**。

## 重跑方法

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd /Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust

# Q1 仓库级
python3 scripts/analyze_error_distribution.py

# Q3 多进程隔离
cargo build --release -p qlexpress-verification
cargo run --release -p qlexpress-verification -- multi-process-isolation 8 30

# Q4 性能基线
cargo run --release -p qlexpress-verification -- load 30 8
```

## 历史

- 2026-09-04：本轮首次落档
