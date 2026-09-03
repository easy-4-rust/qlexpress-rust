<a id="readme-top"></a>

<div align="center">

# QlExpress Rust

**面向 Rust 应用的嵌入式表达式与动态脚本引擎，按行为语义迁移自 Alibaba QLExpress4。**

[![Crates.io](https://img.shields.io/crates/v/qlexpress)](https://crates.io/crates/qlexpress)
[![docs.rs](https://img.shields.io/docsrs/qlexpress)](https://docs.rs/qlexpress)
[![Production Readiness](https://github.com/easy-4-rust/qlexpress-rust/actions/workflows/production-readiness.yml/badge.svg?branch=main)](https://github.com/easy-4-rust/qlexpress-rust/actions/workflows/production-readiness.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-orange)](#环境要求)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

[English](README.md) | [简体中文](README.zh-CN.md)

[快速开始](#快速开始) · [能力矩阵](#能力矩阵) · [架构](#架构) ·
[Java 兼容](#java-兼容) · [验证](#验证) · [文档导航](#文档导航)

</div>

---

> **当前版本**：`0.1.0-alpha.2`<br>
> **成熟度**：Alpha 预览版；`1.0` 之前公共 API 仍可能调整<br>
> **Java 基线**：QLExpress4 `4.2.0-beta`，提交 `9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3`<br>
> **最后核验**：2026-07-30

QlExpress Rust 在 Rust 进程内解析并执行表达式与规则脚本。项目提供解析器、栈式
QVM、与 Java 对应的值及错误语义、自定义函数和操作符、显式宿主类型注册、
安全策略、编译缓存与表达式追踪。

仓库级本地验收和 CI 门禁已经通过。这些证据证明库和验收工具链的状态，但不等于
任意业务宿主已经达到生产可用。真实脚本、真实数据、容量、监控、灰度与回滚仍需在
每个宿主环境中重新验收。

## 为什么选择 QlExpress Rust？

- 无需启动独立服务或 JVM，即可在 Rust 应用中嵌入业务规则。
- 提供类 C/Java 的脚本语法，支持列表、Map、Lambda、函数、循环、动态字符串和宏。
- 可通过 Rust 闭包、自定义操作符和注册类型扩展语言。
- 宿主访问显式可控：Rust 使用 `NativeRegistry`，不复制 JVM 的无限制反射。
- 使用固定的 QLExpress4 基线进行自动差分与官方脚本回放。

### 适用场景

- 计价、优惠、准入、路由、评分、校验等规则；
- 在 Rust 应用中嵌入可配置表达式；
- 将 QLExpress4 规则行为从 Java 迁移到 Rust。

### 明确边界

- 它不是 Java ABI 或 JVM 的替代品。
- 单个 `Express4Runner` 不是 `Send`/`Sync`；多线程应采用“每个 worker 一个 runner”。
- Rust 原生方法和构造器必须显式注册；派生宏不能扫描独立 `impl` 块。
- `0.1.0-alpha.2` 是 Alpha 版本，不代表稳定的 `1.0` 兼容承诺。

## 架构

```text
脚本 + 宿主上下文 + QLOptions
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ Express4Runner                                               │
│  词法/语法解析 → 语法树 → 指令编译                           │
│       │                         │                            │
│       └──────── 编译缓存 ◄──────┘                            │
│                                 ▼                            │
│  函数 / 操作符 / NativeRegistry → QVM + QLambda             │
│                                 │                            │
│                   结果 / Trace / 结构化错误                  │
└──────────────────────────────────────────────────────────────┘
```

核心执行链：

```text
Express4Runner::execute
  → execute_with_context
  → parse_to_definition_with_cache | parse_definition
  → parse_to_syntax_tree
  → QvmInstructionVisitor::compile
  → QvmRuntime::execute
  → QLambdaInner / run_instructions
  → QLResult
```

| Crate | 是否发布 | 职责 |
|:---|:---:|:---|
| `qlexpress` | 是 | 门面、解析器、编译器、QVM、值体系、扩展与安全 |
| `qlexpress-derive` | 是 | 为宿主结构体提供 `#[derive(QLExpressType)]` |
| `qlexpress-verification` | 否 | 差分、回放、并发、负载、fuzz、宿主和灰度验收 |

组件边界、运行流程、安全、失败处理和架构决策见
[架构文档](docs/qlexpress-Architecture.zh_CN.md)。

## 能力矩阵

| 能力 | 状态 | 证据或限制 |
|:---|:---:|:---|
| 表达式、控制流、函数、Lambda、列表和 Map | 已实现 | Alignment 与 Stage 集成测试 |
| 自定义函数、操作符、别名和宏 | 已实现 | `Express4Runner` 公共 API |
| 语法/运行时/超时结构化错误 | 已实现 | 稳定错误码和源码位置 |
| Parse cache 导出与导入 | 已实现 | JSON 模型 v1 与往返测试 |
| 表达式追踪 | 已实现 | 编译期 trace point + 运行时采集 |
| 宿主类型派生宏 | 已实现 | 字段、别名、跳过、类型名；不支持泛型结构体 |
| 原生方法与构造器 | 显式注册 | 不会从 Rust `impl` 块自动发现 |
| 安全策略 | 已实现 | 默认隔离，支持开放、白名单和黑名单 |
| 安全执行入口 | 已实现 | 有限预算、统一能力、租户 LRU、取消 |
| 进程硬隔离 | 可用 | 一次性受监督 Worker；Linux 操作系统内存限制 |
| 多线程执行 | 每 worker 一个 runner | 不支持跨线程共享同一个 runner |
| 跨平台支持 | 尚未声明 | 当前 CI 只在 Ubuntu 执行 |

## 快速开始

### 环境要求

- Rust `1.85` 或更高版本
- 支持 Rust Edition 2021 的 Cargo

添加依赖：

```bash
cargo add qlexpress@0.1.0-alpha.2
```

执行脚本：

```rust
use std::collections::HashMap;

use qlexpress::{DataValue, Express4Runner, QLOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = Express4Runner::new();
    let options = QLOptions::builder().cache(true).build();

    let mut context = HashMap::new();
    context.insert("price".to_string(), DataValue::Double(125.0));
    context.insert("vip".to_string(), DataValue::Bool(true));

    let result = runner.execute(
        "vip ? price * 0.8 : price",
        context,
        &options,
    )?;

    assert_eq!(result.into_result(), DataValue::Double(100.0));
    Ok(())
}
```

运行仓库中的示例：

```bash
cargo run -p qlexpress --example quick_start
```

预期输出：

```text
100.0
```

## 常用扩展

注册 Rust 函数：

```rust
use qlexpress::DataValue;

runner.add_varargs_function("sumAll", |values: &[DataValue]| {
    let total = values.iter().filter_map(|value| match value {
        DataValue::Int(value) => Some(*value),
        _ => None,
    }).sum();
    Ok(DataValue::Int(total))
});
```

暴露宿主结构体：

```rust
use qlexpress::{QLExpressType, QLSecurityStrategy};

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.Order")]
struct Order {
    id: String,
    amount: f64,
    #[qlexpress(skip)]
    internal_note: String,
}

let mut runner = Express4Runner::with_init_options(
    qlexpress::InitOptions::builder()
        .security_strategy(QLSecurityStrategy::open())
        .build(),
);
runner.register_qlexpress_type::<Order>();
```

这里显式使用开放策略只是为了说明示例。面对不可信脚本，应优先使用隔离或最小白名单。
完整示例与限制见[使用指南](docs/Usage-Guide.zh_CN.md)。

## Java 兼容

行为权威为 Alibaba QLExpress4 `4.2.0-beta@9065b9ac`。项目通过官方脚本回放、
共享差分语料、对象/语义矩阵和 Rust 原生集成测试验证兼容性。

| Java 设计 | Rust 设计 | 兼容目标 |
|:---|:---|:---|
| `Express4Runner` | `Express4Runner` | 门面与执行行为 |
| ANTLR 语法树 + Visitor | Rust 词法/语法解析 + Visitor 编译 | 脚本行为，而非解析器内部实现相同 |
| JVM 反射 | `ReflectLoader` + `NativeRegistry` | 显式且经过安全检查的宿主集成 |
| 异常 | `Result<T, QLException>` | 错误分类、错误码、位置和原因 |
| `ConcurrentHashMap<String, Future<...>>` | `RefCell<HashMap<...>>` | 单线程 runner 内的缓存命中语义 |
| Java 注解 | `#[derive(QLExpressType)]` + 显式注册 | 编译期字段元数据；不扫描运行时方法 |
| 动态代理 | 显式闭包/trait 适配器 | 符合 Rust 习惯的替代 |

详细对照：

- [语义迁移对照表](docs/语义迁移对照表.md)
- [对象级对照表](docs/对象级对照表.md)
- [对象名称一致性检查](docs/对象名称一致性检查.md)

## 并发与安全

`Express4Runner` 持有 `Rc`/`RefCell` 状态。应在每个工作线程中创建并配置一个
runner，再在线程内长期复用以获得编译缓存收益。不要给单个 runner 套锁后假定它
获得了与 Java 相同的并发语义。

原生成员的默认策略是 `QLSecurityStrategy::Isolation`。普通 `execute` 保留 Java 兼容的
无限默认值，不是不可信输入沙箱。不可信脚本必须使用 `execute_checked` 或受监督独立进程
Worker。预算、能力白名单、取消、操作系统限制和剩余边界见
[安全沙箱](docs/Security-Sandbox.zh_CN.md)。

## 版本锁定

`0.1.0-alpha.2` 是 Alpha 预览版本。预发布版本之间的公共 API 可能发生变化，且不受
semver 大版本号约束。为避免自动升级导致编译失败，建议在 `Cargo.toml` 中精确锁定版本：

```toml
[dependencies]
qlexpress = "=0.1.0-alpha.2"
qlexpress-derive = "=0.1.0-alpha.2"
```

或使用命令行：

```bash
cargo add qlexpress@=0.1.0-alpha.2
cargo add qlexpress-derive@=0.1.0-alpha.2
```

`=` 前缀强制精确版本匹配。`qlexpress` 与 `qlexpress-derive` 必须使用相同版本——
二者同步发布，混用版本会导致编译错误。

升级前请先阅读 [CHANGELOG.md](CHANGELOG.md) 中的破坏性变更说明。
`1.0` 正式发布后将遵循标准 semver 保证。

## 验证

当前仓库的基础门禁：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

Production Readiness CI 还会执行固定 Java 基线测试、Java/Rust 自动差分、官方脚本
回放、每 worker 独立 runner 的并发验收、确定性安全 fuzz、业务宿主集成、灰度/
回滚模拟、负载验收和 libFuzzer。

2026-07-29 的审计结果包括：803 个 Rust 测试函数且全工作区通过、50/50 差分、
225 个 Maven 测试，以及 151/151 个独立 Java 资源脚本回放。
扩展生产验收还记录了 228/228 个 Java 官方套件用例、16,000 次并发执行、
60 秒 soak、25,000 个确定性安全用例和 31 秒 libFuzzer。命令、实测值与剩余生产边界见
[生产验收](docs/生产验收.md)。

## 文档导航

| 文档 | English | 简体中文 |
|:---|:---:|:---:|
| 迁移技术要求 | [Technical Requirements](docs/QLExpress-Rust-技术要求.md) | [技术要求](docs/QLExpress-Rust-技术要求.md) |
| 迁移测试台账 | [Test Ledgers](docs/迁移测试对照表.md) | [迁移测试对照表](docs/迁移测试对照表.md) |
| 项目概览 | [README](README.md) | [README](README.zh-CN.md) |
| 使用指南 | [Usage Guide](docs/Usage-Guide.md) | [使用指南](docs/Usage-Guide.zh_CN.md) |
| 架构 | [Architecture](docs/qlexpress-Architecture.md) | [架构文档](docs/qlexpress-Architecture.zh_CN.md) |
| API 参考 | [docs.rs](https://docs.rs/qlexpress) | 源码 rustdoc 含中英文说明 |
| 生产验收 | — | [生产验收](docs/生产验收.md) |
| 安全沙箱 | [Security Sandbox](docs/Security-Sandbox.md) | [安全沙箱](docs/Security-Sandbox.zh_CN.md) |

## 开发与发布

日常开发在 `dev` 分支进行，`main` 是发布分支。`main` 中的 `v*` 标签必须先通过
完整 Production Readiness 工作流，之后按顺序发布 `qlexpress-derive` 与
`qlexpress`。

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo publish -p qlexpress-derive --dry-run
cargo publish -p qlexpress --dry-run
```

匹配版本的 `qlexpress-derive` 在 crates.io 可用前，不应发布 `qlexpress`。

## 许可证

项目采用 [Apache License 2.0](LICENSE)。QLExpress 是 Alibaba 项目；本 Rust
迁移项目由 `easy-4-rust` 组织独立维护。

---

<div align="center">

[返回顶部](#readme-top) · [crates.io](https://crates.io/crates/qlexpress) ·
[docs.rs](https://docs.rs/qlexpress) ·
[Issues](https://github.com/easy-4-rust/qlexpress-rust/issues)

</div>
