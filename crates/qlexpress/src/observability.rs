//! Feature-gated tracing observability for the qlexpress engine.
//!
//! When the `tracing` Cargo feature is **enabled**, `tracing` spans and events
//! are emitted at parse / compile / execute boundaries so that production
//! operators can observe latency, error rates, and throughput through standard
//! `tracing` subscribers (e.g. `tracing-subscriber`).
//!
//! When the `tracing` feature is **disabled** (the default), every
//! `#[cfg(feature = "tracing")]` block is removed by the compiler, resulting
//! in zero runtime cost and zero additional dependencies.
//!
//! # Instrumentation points
//!
//! | Boundary | File | What is recorded |
//! |---|---|---|
//! | `execute_with_context` | `express4_runner/execution.rs` | script length, success/error |
//! | `execute_definition` | `express4_runner/execution.rs` | elapsed ms, success/error |
//! | `run_instructions` | `runtime/qvm_runtime.rs` | QVM loop elapsed ms, instruction count |
//! | `parse_definition` | `express4_runner/compilation.rs` | compile elapsed ms |
//! | `parse_to_definition_with_cache` | `express4_runner/compilation.rs` | cache hit/miss, compile elapsed ms |
//!
//! All instrumentation uses `#[cfg(feature = "tracing")]` directly at the call
//! site (no custom macros) for maximum clarity and zero abstraction overhead.
