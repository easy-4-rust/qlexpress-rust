# QLExpress Rust 性能基线报告

日期：2026-09-05
分支：dev（HEAD=5ace899）
版本：0.1.0-beta.0

---

## 测试环境

| 项目 | 值 |
|------|-----|
| 操作系统 | macOS 25.6.0 (Darwin Kernel 25.6.0) arm64 |
| 硬件 | Apple M1（MacBook Pro） |
| Rust | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| Cargo | cargo 1.97.1 (c980f4866 2026-06-30) |
| Release profile | 默认（无自定义 `[profile.release]`，无 LTO） |

---

## 执行命令

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd /Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust
cargo build --release -p qlexpress-verification
cargo run --release -p qlexpress-verification -- load 30 8
```

参数：30 秒时长，8 线程并发（runner-per-worker 模型，每个线程独立 `Express4Runner`）。

---

## 实跑结果

### 第一次运行

```json
{
  "threads": 8,
  "duration_seconds": 30,
  "executions": 1552607,
  "errors": 0,
  "throughput_ops_sec": 51753.57,
  "p50_micros": 15,
  "p95_micros": 545,
  "p99_micros": 914,
  "max_micros": 72966
}
```

### 第二次运行

```json
{
  "threads": 8,
  "duration_seconds": 30,
  "executions": 1601106,
  "errors": 0,
  "throughput_ops_sec": 53370.20,
  "p50_micros": 17,
  "p95_micros": 535,
  "p99_micros": 835,
  "max_micros": 40222
}
```

### 稳定性校验

| 指标 | 第一次 | 第二次 | 差异 |
|------|--------|--------|------|
| ops/s | 51,753.57 | 53,370.20 | +3.1% |
| p99 (µs) | 914 | 835 | -8.6% |

p99 差异 8.6% < 20% 阈值，两次运行稳定，无需第三次。

---

## 汇总数据（两次中位数）

| 指标 | 本次实跑值 |
|------|-----------|
| **吞吐量** | **52,562 ops/s** |
| **总执行次数** | ~1,576,857（30s × 2 次平均） |
| **p50 延迟** | **16 µs** |
| **p95 延迟** | **540 µs** |
| **p99 延迟** | **874 µs** (0.874 ms) |
| **max 延迟** | **56.6 ms**（取中位数；单次最高 72.9 ms） |
| **错误数** | **0** |
| **线程数** | 8 |
| **单次时长** | 30 秒 |

---

## 与历史数据对比

| 日期 | 版本 | 时长 | ops/s | p99 | max | 错误 |
|------|------|------|-------|-----|-----|------|
| 2026-07-27 | pre-beta | 60s | 11,899 | 3,460 µs | — | 0 |
| 2026-07-29 | pre-beta | 15s | 10,140 | 4,859 µs | — | 0 |
| 2026-07-30 | pre-beta | 15s | 11,064 | 4,074 µs | — | 0 |
| 2026-08-08 | beta.0 验收 | 15s | 8,596 | 5,081 µs | — | 0 |
| 2026-09-03 | beta.0 晋级 | 15s | 55,135 | 887 µs | 24.9 ms | 0 |
| **2026-09-05** | **dev (5ace899)** | **30s** | **52,562** | **874 µs** | **56.6 ms** | **0** |

### 趋势分析

- **吞吐量**：从 08-08 的 8,596 ops/s 飙升至 09-03 的 55,135 ops/s（+541%），本轮 52,562 ops/s 与 09-03 持平（-4.7%，在正常波动范围内）。
- **p99 延迟**：从 08-08 的 5.081 ms 降至 09-03 的 887 µs（-82.6%），本轮 874 µs 继续持平（-1.5%）。
- **稳定性**：本轮两次运行 p99 波动仅 8.6%，吞吐量波动 3.1%，均在合理范围内。
- **max 延迟**：本轮 max 值 56.6 ms 高于 09-03 的 24.9 ms，但 max 是长尾指标，单次偶发不影响整体性能。

---

## 对业务验收的启示

### 1. 单次 execute_definition 延迟上限

- **p99 = 874 µs（0.874 ms）**：99% 的请求在 1 ms 以内完成。
- **p50 = 16 µs**：一半请求仅需 16 微秒，体现了编译缓存（parsecache）的命中收益。
- 对于规则引擎场景，1 ms 以内的 p99 是非常优秀的水平。

### 2. 并发能力上限

- **8 线程下达到 52,562 ops/s**：即每秒可处理约 5.2 万次脚本执行。
- 业务要求 100 次/秒的并发，实测能力是需求的 **525 倍**，留有极大余量。
- 假设线性扩展（runner-per-worker 模型下各线程独立，无锁竞争），32 线程理论上可达 ~210K ops/s。

### 3. 与 Java 引擎的参考差距

根据仓库历史数据，Java 版 QLExpress4 在同一硬件上的典型性能量级约为数千 ops/s（08-08 验收时 Rust 为 8,596 ops/s，当时已接近或超过 Java 水平）。经过 09-03 的优化后，Rust 版本吞吐量提升约 6 倍，p99 降低约 5.7 倍，预期与 Java 引擎的差距在 **5-10 倍** 量级（Rust 优于 Java）。

> 注：未在本次测试中直接对比 Java，上述为基于历史数据的推算。精确对比需在同一硬件、同一脚本集、同一并发度下实测。

### 4. 业务验收结论

| 验收维度 | 通过标准 | 实际 | 结论 |
|----------|----------|------|------|
| 错误数 | 0 | 0 | 通过 |
| p99 延迟 | < 250 ms（历史标准） | 0.874 ms | 通过（远低于阈值） |
| 吞吐量 | > 100 ops/s | 52,562 ops/s | 通过（525x 余量） |
| 稳定性 | 两次 p99 差异 < 20% | 8.6% | 通过 |

---

## 复现步骤

```bash
# 1. 环境准备
export PATH="$HOME/.cargo/bin:$PATH"
cd /Users/wandl/workspaces/workspace-github-easy-4-rust/qlexpress-rust

# 2. 构建
cargo build --release -p qlexpress-verification

# 3. 运行负载测试
cargo run --release -p qlexpress-verification -- load 30 8
```

输出为单行 JSON，包含所有指标字段。
