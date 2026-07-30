# QlExpress Rust Usage Guide

> **Purpose:** Provide a source-backed path from first evaluation to safe host integration.<br>
> **Applies to:** `qlexpress 0.1.0-alpha.2`<br>
> **Rust baseline:** MSRV 1.85, Edition 2021<br>
> **Last verified:** 2026-07-27<br>
> **Status:** Alpha documentation; APIs may change before `1.0`

[简体中文](Usage-Guide.zh_CN.md) | [Project README](../README.md) |
[Architecture](qlexpress-Architecture.md)

## 1. Mental model

An application owns an `Express4Runner`, registers the host capabilities a script may use, and
executes scripts with a context and per-run `QLOptions`.

```text
configure runner once
  → register functions/operators/types
  → validate or precompile scripts
  → execute(script, context, options)
  → inspect QLResult or QLException
```

Runner initialization and execution options have different lifetimes:

| Type | Lifetime | Examples |
|:---|:---|:---|
| `InitOptions` | Runner construction | security policy, tracing support, interpolation, debug |
| `QLOptions` | One execution policy, reusable value | timeout, cache, attachments, array limit, tracing |
| `CheckOptions` | Static validation | operator allow/deny set, disable function calls |

## 2. Install and run

```bash
cargo add qlexpress@0.1.0-alpha.2
```

```rust
use std::collections::HashMap;

use qlexpress::{DataValue, Express4Runner, QLOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = Express4Runner::new();
    let options = QLOptions::builder().cache(true).build();
    let mut context = HashMap::new();
    context.insert("a".into(), DataValue::Int(19));
    context.insert("b".into(), DataValue::Int(23));

    let value = runner.execute("a + b", context, &options)?.into_result();
    assert_eq!(value, DataValue::Int(42));
    Ok(())
}
```

From this repository:

```bash
cargo run -p qlexpress --example quick_start
```

Expected output:

```text
100.0
```

## 3. Scripts and values

Representative language features:

```text
// arithmetic and conditional
score = base + bonus;
score >= 80 ? 'PASS' : 'REVIEW'

// list, map and field access
items = [1, 2, 3];
result = {'count': items.size(), 'first': items[0]};

// lambda and function
twice = x -> x * 2;
function add(a, b) { return a + b; }
add(twice(10), 22)

// control flow
sum = 0;
for (i = 1; i <= 4; i = i + 1) { sum = sum + i; }
sum
```

The host boundary uses `DataValue`:

| Rust variant | Script/Java-style meaning |
|:---|:---|
| `Null`, `Bool`, `Char`, `Str` | null, boolean, character, string |
| `Byte`, `Short`, `Int`, `Long` | integral values |
| `Float`, `Double`, `BigInt`, `BigDec` | floating and arbitrary-precision values |
| `List`, `Array`, `Map` | mutable reference-semantic collections |
| `Lambda` | compiled script lambda |
| `Object` | explicitly registered host object |

Collections and host objects use `Rc<RefCell<...>>` internally to preserve Java-like reference
semantics inside one runner thread.

## 4. Execution options

```rust
let options = QLOptions::builder()
    .cache(true)
    .timeout_millis(500)
    .max_arr_length(10_000)
    .avoid_null_pointer(true)
    .build();
```

| Option | Default | Effect |
|:---|:---:|:---|
| `precise` | `false` | Prefer precise decimal evaluation where supported |
| `pollute_user_context` | `false` | Allow script-defined globals to update the host context |
| `timeout_millis` | `-1` | `<= 0` means no engine timeout |
| `attachments` | empty | Host-only metadata passed to extension functions |
| `cache` | `false` | Reuse the compiled definition for the same script |
| `avoid_null_pointer` | `false` | Enable Java-compatible null-avoidance behavior |
| `max_arr_length` | `-1` | Limit arrays created by scripts |
| `trace_expression` | `false` | Collect expression traces when also enabled in `InitOptions` |
| `short_circuit_disable` | `false` | Disable logical short-circuiting |

An engine timeout is a cooperative script timeout enforced by inserted QVM instructions. A host
should still apply request deadlines, input limits, and process-level isolation appropriate to its
threat model.

## 5. Custom functions

Use a closure when the function needs the execution context and typed parameters:

```rust
use qlexpress::runtime::{parameters::Parameters, qcontext::QContext};
use qlexpress::DataValue;

runner.add_function(
    "double",
    |_context: &mut dyn QContext, params: &Parameters| {
        match params.get_value(0) {
            DataValue::Int(value) => Ok(DataValue::Int(value * 2)),
            _ => Ok(DataValue::Null),
        }
    },
);
```

Use `add_varargs_function` for a simple slice-based function:

```rust
runner.add_varargs_function("sumAll", |params: &[DataValue]| {
    let sum = params.iter().fold(0, |sum, value| match value {
        DataValue::Int(value) => sum + value,
        _ => sum,
    });
    Ok(DataValue::Int(sum))
});
```

Registration uses `putIfAbsent` semantics: the method returns `false` if a function with the same
name already exists. `batch_add_function` reports successful and failed names separately.

## 6. Custom operators and aliases

Custom operators mutate runner configuration and therefore require `&mut Express4Runner`:

```rust
let mut runner = Express4Runner::new();

assert!(runner.add_operator_bi("**", |left, right| match (left, right) {
    (DataValue::Int(base), DataValue::Int(exp)) => {
        DataValue::Int(base.pow(exp as u32))
    }
    _ => DataValue::Null,
}));

assert!(runner.add_operator_alias("plus", "+"));
```

Avoid replacing built-in operators unless migration compatibility requires it. Replacement changes
the meaning of every script evaluated by that runner.

## 7. Host structs with `QLExpressType`

The derive macro generates native type metadata and field access for a named-field, non-generic
struct:

```rust
use qlexpress::runtime::member::QLExpressNativeType;
use qlexpress::{DataValue, Express4Runner, InitOptions, QLExpressType, QLOptions};
use qlexpress::QLSecurityStrategy;

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.Order")]
struct Order {
    id: String,
    amount: f64,
    #[qlexpress(alias("level"))]
    customer_level: i64,
    #[qlexpress(skip)]
    internal_note: String,
}

let mut runner = Express4Runner::with_init_options(
    InitOptions::builder()
        .security_strategy(QLSecurityStrategy::open())
        .build(),
);
runner.register_qlexpress_type::<Order>();

let order = Order {
    id: "O-1001".into(),
    amount: 1200.0,
    customer_level: 4,
    internal_note: "not exposed".into(),
};
let mut context = std::collections::HashMap::new();
context.insert("order".into(), order.into_data_value());

let result = runner.execute(
    "order.amount >= 1000.0 && order.level >= 4",
    context,
    &QLOptions::default(),
)?;
assert_eq!(result.into_result(), DataValue::Bool(true));
# Ok::<(), qlexpress::QLException>(())
```

Supported helper attributes:

| Location | Attribute | Meaning |
|:---|:---|:---|
| Struct | `name = "..."` | Override the registered type name |
| Struct | `expose_fields` | Also expose fields through method-style resolution |
| Field | `skip` | Do not expose this field |
| Field | `readonly` | Generate read access only; script assignment and classified-object population reject writes |
| Field | `alias("a", "b")` | Add alternative field names |

Current derive limits: named-field structs only, no generic structs, and no automatic method or
constructor discovery. Register those through `NativeRegistry` or the runner's explicit APIs.

## 8. Security and validation

For untrusted input, use `Express4Runner::execute_checked` with
`SandboxProfile::secure()`. Plain `execute` intentionally retains Java-compatible unlimited
defaults. The checked entry combines static validation, finite parse/compile/runtime budgets,
capability allowlisting, cancellation, and a tenant-bounded LRU. Adversarial input should
additionally run through `qlexpress-sandbox-worker`; see
[Security Sandbox](Security-Sandbox.md).

The default native-member policy is isolation:

```rust
let init = InitOptions::builder()
    .security_strategy(QLSecurityStrategy::isolation())
    .build();
let runner = Express4Runner::with_init_options(init);
```

Use `QLSecurityStrategy::white_list` for the smallest set of native members required by a script.
Use `open()` only for trusted inputs or tightly controlled examples.

Static validation can restrict syntax before execution:

```rust
use std::collections::HashSet;
use qlexpress::operator::OperatorCheckStrategy;
use qlexpress::CheckOptions;

let allowed: HashSet<String> = ["+", "*"].into_iter().map(String::from).collect();
let checks = CheckOptions::builder()
    .operator_check_strategy(OperatorCheckStrategy::whitelist(allowed))
    .disable_function_calls(true)
    .build();

runner.check("1 + 2 * 3", &checks)?;
# Ok::<(), qlexpress::QLSyntaxException>(())
```

Validation does not make arbitrary scripts safe by itself. Combine it with member policy, limits,
timeouts, fuzzing, and deployment isolation.

## 9. Compile cache and portable parse cache

`QLOptions::cache(true)` caches the compiled definition by exact script text inside the runner.
The cache is local to that runner and thread.

For a serializable cache:

```rust
let exported = runner.export_parse_cache("a + b")?;
let loaded = runner.import_parse_cache(&exported)?;
let result = runner.execute_with_loaded_cache(
    &loaded,
    std::rc::Rc::new(qlexpress::runtime::context::EmptyContext::new()),
    &QLOptions::default(),
)?;
# Ok::<(), qlexpress::QLException>(())
```

A loaded cache is bound to the importing runner. Use `set_parse_cache` to prewarm the runner's
normal compile cache. Treat cache JSON as versioned data: verify model version, producer version,
script hash, and trusted provenance.

## 10. Expression tracing and static analysis

Tracing must be enabled at runner initialization and for the execution:

```rust
let runner = Express4Runner::with_init_options(
    InitOptions::builder().trace_expression(true).build(),
);
let options = QLOptions::builder().trace_expression(true).build();
let result = runner.execute("a > 10 && b < 20", context, &options)?;
for trace in result.expression_traces() {
    println!("{trace:?}");
}
```

Static analysis APIs include:

- `get_out_var_names` — variables supplied by the host;
- `get_out_var_attrs` — accessed property paths;
- `get_out_function_names` — external functions;
- `get_expression_trace_points` — expression trace tree without execution;
- `parse_to_syntax_tree` and `parse_to_instructions` — diagnostics and tooling support.

## 11. Errors

All execution failures use `QLException`. Inspect stable fields instead of parsing the formatted
message:

```rust
match runner.execute(script, context, &options) {
    Ok(result) => println!("{:?}", result.result()),
    Err(error) => eprintln!(
        "kind={:?} code={} line={} col={} reason={}",
        error.kind(),
        error.error_code(),
        error.line_no(),
        error.col_no(),
        error.reason(),
    ),
}
```

`QLExceptionKind` distinguishes syntax, runtime, and timeout failures. `check` returns the narrower
`QLSyntaxException`.

## 12. Concurrency model

Do not share one runner between threads. Create and configure a runner inside each worker:

```text
request dispatcher
  ├── worker 1 → runner 1 + cache 1
  ├── worker 2 → runner 2 + cache 2
  └── worker N → runner N + cache N
```

The repository acceptance command is:

```bash
cargo run -p qlexpress-verification -- concurrency 8 2000
```

## 13. Production checklist

- Pin the crate version and record the Java compatibility baseline if migrating.
- Inventory and review every function, operator, type, field, method, and constructor exposed.
- Use isolation or an allowlist; avoid `open()` for untrusted scripts.
- Set script size, context size, timeout, array, memory, and request concurrency limits.
- Replay real, sanitized scripts and compare outputs before cutover.
- Run load and soak tests on production-like hardware with the real worker model.
- Capture error codes, latency, cache behavior, and business outcomes in host telemetry.
- Canary the new engine against the stable decision path and define automatic rollback criteria.

Repository commands and recorded results are in
[Production Acceptance](生产验收.md). They do not replace host-specific staging and canary evidence.

---

**Document version:** 1.0.0<br>
**Created:** 2026-07-27<br>
**Last updated:** 2026-07-27<br>
**Status:** Ready for review
