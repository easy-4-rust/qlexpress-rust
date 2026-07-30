<a id="readme-top"></a>

<div align="center">

# QlExpress Rust

**An embeddable Rust expression and dynamic scripting engine, behaviorally ported from Alibaba QLExpress4.**

[![Crates.io](https://img.shields.io/crates/v/qlexpress)](https://crates.io/crates/qlexpress)
[![docs.rs](https://img.shields.io/docsrs/qlexpress)](https://docs.rs/qlexpress)
[![Production Readiness](https://github.com/easy-4-rust/qlexpress-rust/actions/workflows/production-readiness.yml/badge.svg?branch=main)](https://github.com/easy-4-rust/qlexpress-rust/actions/workflows/production-readiness.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-orange)](#requirements)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

[English](README.md) | [简体中文](README.zh-CN.md)

[Quick start](#quick-start) · [Capabilities](#capabilities) · [Architecture](#architecture) ·
[Compatibility](#java-compatibility) · [Verification](#verification) · [Documentation](#documentation)

</div>

---

> **Release:** `0.1.0-alpha.2`<br>
> **Maturity:** alpha preview; APIs may change before `1.0`<br>
> **Java baseline:** QLExpress4 `4.2.0-beta`, commit `9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3`<br>
> **Last verified:** 2026-07-30

QlExpress Rust evaluates expressions and rule scripts inside a Rust process. It provides a parser, a
stack-based QVM, Java-compatible value and error semantics, custom functions and operators,
explicit native-type registration, security policies, compile caches, and expression tracing.

The repository has passed its repeatable local and CI readiness gates. That is evidence for the
library and its harness—not proof that an arbitrary business deployment is production-ready.
Real scripts, data, capacity limits, monitoring, rollout, and rollback must still be accepted in
each host environment.

## Why QlExpress Rust?

- Embed business rules without starting a separate service or JVM.
- Use a familiar C/Java-like expression language with lists, maps, lambdas, functions, loops,
  dynamic strings, macros, and structured errors.
- Extend the language through Rust closures, custom operators, and registered host types.
- Keep host access explicit: Rust uses `NativeRegistry` instead of unrestricted JVM reflection.
- Compare behavior against a pinned QLExpress4 baseline with differential and replay tests.

### Good fit

- pricing, promotion, eligibility, routing, scoring, and validation rules;
- configurable expressions embedded in a Rust application;
- teams migrating QLExpress4 rule behavior from Java to Rust.

### Boundaries

- This is not a Java ABI/JVM replacement.
- A single `Express4Runner` is not `Send` or `Sync`; use one runner per worker thread.
- Rust native methods and constructors are registered explicitly; derive cannot inspect `impl`
  blocks.
- `0.1.0-alpha.2` is an alpha release, not a stable `1.0` compatibility promise.

## Architecture

```text
script + host context + QLOptions
                 │
                 ▼
┌──────────────────────────────────────────────────────────────┐
│ Express4Runner                                               │
│  lexer/parser → syntax tree → instruction compiler           │
│       │                              │                       │
│       └──────── compile cache ◄──────┘                       │
│                                      ▼                       │
│  functions / operators / NativeRegistry → QVM + QLambda     │
│                                      │                       │
│                       result / trace / structured error       │
└──────────────────────────────────────────────────────────────┘
```

The execution path is:

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

| Crate | Published | Responsibility |
|:---|:---:|:---|
| `qlexpress` | Yes | Public facade, parser, compiler, QVM, values, extensions, security |
| `qlexpress-derive` | Yes | `#[derive(QLExpressType)]` for registered host structs |
| `qlexpress-verification` | No | Differential, replay, concurrency, load, fuzz, host, and canary harness |

See [Architecture](docs/qlexpress-Architecture.md) for component boundaries, runtime flows,
security, failure handling, and architecture decisions.

## Capabilities

| Capability | Status | Evidence / limit |
|:---|:---:|:---|
| Expressions, control flow, functions, lambdas, lists, maps | Implemented | Alignment and stage integration tests |
| Custom functions, operators, aliases, and macros | Implemented | Public `Express4Runner` APIs |
| Structured syntax/runtime/timeout errors | Implemented | Stable error codes and source positions |
| Parse-cache export/import | Implemented | JSON model v1 and round-trip tests |
| Expression tracing | Implemented | Compile-time trace points plus runtime collection |
| Host-type derive | Implemented | Fields, aliases, skip, name override; no generic structs |
| Native methods and constructors | Explicit registration | Not discovered from Rust `impl` blocks |
| Security policy | Implemented | Isolation default; open, allowlist, and denylist modes |
| Checked sandbox execution | Implemented | Finite budgets, unified capabilities, tenant LRU, cancellation |
| Hard process isolation | Available | Supervised one-shot worker; Linux OS memory limit |
| Multi-thread execution | Runner per worker | Sharing one runner across threads is unsupported |
| Cross-platform support | Not yet claimed | CI currently executes on Ubuntu only |

## Quick start

### Requirements

- Rust `1.85` or newer
- Cargo with Rust Edition 2021 support

Add the crate:

```bash
cargo add qlexpress@0.1.0-alpha.2
```

Evaluate a script:

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

Run the repository example:

```bash
cargo run -p qlexpress --example quick_start
```

Expected output:

```text
100.0
```

## Common extensions

Register a Rust function:

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

Expose a host struct:

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

The open policy is shown to make the example explicit. Prefer isolation or a narrow allowlist for
untrusted scripts. The full examples and limits are in the
[Usage Guide](docs/Usage-Guide.md).

## Java compatibility

The behavioral authority is Alibaba QLExpress4 `4.2.0-beta` at commit `9065b9ac`. Compatibility
is verified through upstream script replay, a shared differential corpus, object/semantic
matrices, and Rust-native integration tests.

| Java design | Rust design | Compatibility intent |
|:---|:---|:---|
| `Express4Runner` | `Express4Runner` | Facade and execution behavior |
| ANTLR syntax tree + visitor | Rust lexer/parser + visitor compiler | Script behavior, not parser implementation identity |
| JVM reflection | `ReflectLoader` + `NativeRegistry` | Explicit, security-checked host integration |
| Exceptions | `Result<T, QLException>` | Error category, code, location, and reason |
| `ConcurrentHashMap<String, Future<...>>` | `RefCell<HashMap<...>>` | Cache-hit semantics in a single-thread runner |
| Java annotations | `#[derive(QLExpressType)]` + explicit registration | Compile-time field metadata; no runtime method scanning |
| Dynamic proxies | Explicit closure/trait adapters | Idiomatic Rust replacement |

Detailed mappings:

- [Semantic migration matrix](docs/语义迁移对照表.md)
- [Object-level mapping](docs/对象级对照表.md)
- [Name consistency audit](docs/对象名称一致性检查.md)

## Concurrency and security

`Express4Runner` owns `Rc`/`RefCell` state. Create and configure one runner per worker thread, then
reuse it inside that worker to benefit from compile caching. Do not wrap one runner in a mutex and
assume Java-equivalent concurrency.

The default native-member policy is `QLSecurityStrategy::Isolation`. Plain `execute` preserves
Java-compatible unlimited defaults and is not an untrusted-input sandbox. Untrusted scripts must
use `execute_checked` or the supervised process worker. See the
[Security Sandbox](docs/Security-Sandbox.md) for budgets, capabilities, cancellation, OS limits,
and residual boundaries.

## Verification

The current repository gates include:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

Production-readiness CI additionally runs the pinned Java suite, Java/Rust differential tests,
official script replay, runner-per-worker concurrency, deterministic security fuzzing, a
business-host scenario, canary/rollback simulation, load acceptance, and libFuzzer.

The recorded 2026-07-29 audit found 803 Rust test functions and passed the full workspace,
50/50 differential cases, 225 Maven tests, and 151/151 independent Java resource-script replays.
The extended production run also records 228/228 Java official-suite cases, 16,000 concurrent
executions, a 60-second soak, 25,000 deterministic security cases, and a 31-second libFuzzer run.
Read the commands, measurements, and remaining deployment boundary in
[Production Acceptance](docs/生产验收.md).

## Documentation

| Document | English | 简体中文 |
|:---|:---:|:---:|
| Migration technical requirements | [Technical Requirements](docs/QLExpress-Rust-技术要求.md) | [技术要求](docs/QLExpress-Rust-技术要求.md) |
| Migration test ledgers | [Test Ledgers](docs/迁移测试对照表.md) | [迁移测试对照表](docs/迁移测试对照表.md) |
| Project overview | [README](README.md) | [README](README.zh-CN.md) |
| Usage guide | [Usage Guide](docs/Usage-Guide.md) | [使用指南](docs/Usage-Guide.zh_CN.md) |
| Architecture | [Architecture](docs/qlexpress-Architecture.md) | [架构文档](docs/qlexpress-Architecture.zh_CN.md) |
| API reference | [docs.rs](https://docs.rs/qlexpress) | Source rustdoc includes bilingual notes |
| Production acceptance | — | [生产验收](docs/生产验收.md) |
| Security sandbox | [Security Sandbox](docs/Security-Sandbox.md) | [安全沙箱](docs/Security-Sandbox.zh_CN.md) |

## Development and release

Development happens on `dev`; `main` is the release branch. A `v*` tag contained in `main` runs
the complete readiness workflow before publishing `qlexpress-derive` and then `qlexpress`.

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo publish -p qlexpress-derive --dry-run
cargo publish -p qlexpress --dry-run
```

Do not publish `qlexpress` before the matching exact version of `qlexpress-derive` is available.

## License

Licensed under the [Apache License, Version 2.0](LICENSE). QLExpress is an Alibaba project; this
Rust port is maintained independently by the `easy-4-rust` organization.

---

<div align="center">

[Back to top](#readme-top) · [crates.io](https://crates.io/crates/qlexpress) ·
[docs.rs](https://docs.rs/qlexpress) ·
[Issues](https://github.com/easy-4-rust/qlexpress-rust/issues)

</div>
