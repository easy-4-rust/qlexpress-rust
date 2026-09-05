# 业务验收

本目录记录 qlexpress-rust **从工程就绪跨到生产就绪**的实测证据。仅依赖仓库自身资源（不要求用户提供业务脚本）。

## 已完成的验收面

### Q1-biz: 错误码近似业务覆盖

- 脚本：[scripts/generate_error_code_coverage.py](/Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust/scripts/generate_error_code_coverage.py) + 68 条合成语料
- 报告：[business-synthetic-coverage.md](/Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust/docs/业务验收/business-synthetic-coverage.md)
- 关键结论：66 错误码**全部构造了理论触发场景**（覆盖从 9/66 → 66/66）；59 条 QLExpress 脚本可触发，9 条需非脚本机制（parse cache 损坏 / 注册函数）
- **能告诉你什么**：哪些错误码**可能**被业务踩到（按构造难度的优先级排序）
- **不能告诉你什么**：业务真实频率——只有你的脚本能告诉我
- 17 个 Python 单元测试

### Q2: 调用栈深度（仓库级实测）

- Harness: `cargo run --release -p qlexpress-verification -- stack-depth-probe`
- 报告：[stack-depth-probe.md](/Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust/docs/业务验收/stack-depth-probe.md)
- 三类测试：
  - **函数递归深度**：max_call_depth=128 精确执行；N≤128 OK，N≥129 触发 `SANDBOX_CALL_DEPTH_EXCEEDED`
  - **操作数栈**：N≤114 OK；**N=115 触发 `PROCESS_STACK_OVERFLOW`**（Rust 进程栈）
  - **嵌套 try-catch**：N≤100 OK；**N=105 同样触发进程栈溢出**
- **关键发现**：QVM 操作数栈（max_stack_size + try_push）正确性 100%；但解析器递归下降**深度 ~115 即进程栈溢出**——**未修，新 P0**

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

### Q1-biz: 业务脚本真实错误码分布

- 仓库级 Q1 给出 295 差分基线，但**业务真实错误谱**未知
- 建议：用户提供 100-1000 条脱敏业务脚本到 `scripts/business-corpus/`，跑 `scripts/analyze_error_distribution.py --corpus scripts/business-corpus/`，对比仓库基线
- **本轮（不依赖业务脚本）的近似替代**：见 [business-synthetic-coverage.md](business-synthetic-coverage.md)——66 错误码全部构造了合成触发场景

### Q2: 调用栈深度

- 状态：**完成 + P0 已修**——见 [stack-depth-probe.md](stack-depth-probe.md)
- **关键发现**（修复前）：操作数栈预算 100% 正确；但 N=115 触发 `PROCESS_STACK_OVERFLOW`（解析器递归下降无深度限制）
- **P0 修复**（2026-09-04 提交 `*P0-parser-depth*`）：新增错误码 `PARSE_AST_DEPTH_EXCEEDED`，6 个递归入口加 RAII DepthGuard（thread_local + Cell），`MAX_PARSE_DEPTH=100`
- **修复后实测**：N=200/500/1000 全部返回 `PARSE_AST_DEPTH_EXCEEDED`（depth 101）而非进程崩溃；延迟 < 5ms

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
