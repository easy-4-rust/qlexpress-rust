# qlexpress

Alibaba [QLExpress4](https://github.com/alibaba/QLExpress) 动态脚本引擎的 Rust 全量语义迁移
(Rust 首发版本 `0.1.0-alpha.1`，逐类对齐 `com.alibaba.qlexpress4`
`4.2.0-beta`，基线提交 `9065b9ac`)。

## 与 QLExpress4 的对应关系

| Java (`com.alibaba.qlexpress4`) | Rust (`qlexpress`) |
| --- | --- |
| `Express4Runner` | `express4_runner::Express4Runner`(门面,见下) |
| `InitOptions` / `QLOptions` / `QLResult` / `CheckOptions` | `init_options` / `ql_options` / `ql_result` / `check_options` |
| `ClassSupplier` / `DefaultClassSupplier` | `class_supplier` / `default_class_supplier`(显式注册替代 `Class.forName`) |
| `aparser.*`(QLexer/QLParser/QvmInstructionVisitor 等) | `aparser/`(词法、递归下降语法树、编译为 QVM 指令) |
| `runtime.*`(QvmRuntime/QLambda/Value/function/context) | `runtime/`(QVM、Lambda、值体系、函数、上下文) |
| `runtime.operator.*` | `runtime/operator/`(内建操作符全量 + 自定义操作符) |
| `security.QLSecurityStrategy` | `security`(open/isolation/黑/白名单) |
| `api.parsecache.*` | `api/parsecache/`(serde JSON 序列化编译缓存) |
| `exception.QLException` 族 | `exception/`(统一 `QLException` + kind: Syntax/Runtime/Timeout) |
| `proxy.QLambdaInvocationHandler` | `proxy/`(Java 动态代理 → 显式闭包/trait 适配器,见文件头注释) |
| `enums.AccessMode` | `enums::AccessMode` |

## 快速上手

```toml
[dependencies]
qlexpress = "=0.1.0-alpha.1"
```

```rust
use std::collections::HashMap;
use qlexpress::{DataValue, Express4Runner, QLExpressType, QLOptions};

let runner = Express4Runner::new();
let options = QLOptions::builder().build();

// 执行脚本(Map 上下文:key 即脚本变量名)
let mut context = HashMap::new();
context.insert("a".to_string(), DataValue::Int(19));
context.insert("b".to_string(), DataValue::Int(23));
let result = runner.execute("a + b", context, &options)?;
assert_eq!(result.into_result(), DataValue::Int(42));

// 注册自定义函数(Java: addFunction(name, (qContext, params) -> ...))
runner.add_function("dbl", |_ctx, params| {
    match params.get_value(0) {
        DataValue::Int(v) => Ok(DataValue::Int(v * 2)),
        _ => Ok(DataValue::Null),
    }
});
let r = runner.execute("dbl(21)", HashMap::new(), &options)?;
assert_eq!(r.into_result(), DataValue::Int(42));
```

更多能力:`add_varargs_function` / `batch_add_function`(部分失败语义)/
`add_function_of_class_method` / `add_static_method`(经 NativeRegistry)/
`add_operator` / `replace_operator` / `add_alias` / `check`(操作符黑白名单)/
`set_security_strategy`(成员访问黑白名单)/ `add_macro` /
`add_compile_time_function` / `get_out_var_names` / `get_out_function_names` /
`export_parse_cache` → `import_parse_cache` → `execute_with_cache`。端到端示例见
`tests/stage5c_runner.rs`。

## 构建 / 测试

```bash
cargo build
cargo test          # 全量:lib 单测 + tests/ 各 Stage 端到端
cargo clippy --all-targets
```

## 当前语义对齐范围与 Rust 实现差异

- **数值**:`BigInteger` 使用 `num_bigint::BigInt` 任意精度实现；
  `BigDecimal` 以十进制字符串存储、按需解析。
- **反射替代**(SPEC §4):Java 的 `Class.forName`/反射成员解析改为显式
  `NativeRegistry` 注册(类型、构造器、方法、字段)；`@QLFunction` /
  `@QLAlias` 扫描职责由 `#[derive(QLExpressType)]` 在编译期生成，
  宿主也可通过 `addObjFunction`/`addStaticFunction` 对应的注册 API 显式接线。
- **安全策略**:作用于构造器、注册表成员、动态宿主对象以及内建 JDK
  兼容成员分派，对齐 Java `ReflectLoader.check`；不通过时按「成员不存在」
  报错。扩展函数先于隔离策略解析，`Express4Runner` 的宿主字段读取保留
  Java `skipSecurity=true` 边界。
- **并发编译缓存**:Java `ConcurrentHashMap<String, Future<QCompileCache>>`
  → 单线程 `Rc` 体系下的 `RefCell<HashMap>`(命中语义一致)。
- **表达式 trace**:`TraceExpressionVisitor`、`TracePointTree`、
  `ExpressionTrace` 与 `QTraces` 已形成编译期标注、运行时采集和嵌套
  lambda 追踪闭环，`get_expression_trace` 返回可序列化追踪结果。
- **动态代理**:`proxy.QLambdaInvocationHandler` 以显式闭包/trait 适配器
  替代 Java 运行时接口代理。
- **运行时反射改注册表**:Rust 无 JVM 运行时反射，等价职责由
  `ReflectLoader` 门面和 `NativeRegistry` 承担，注册在 `&mut runner` 上进行。
- **#[derive(QLExpressType)]**:过程宏自动生成 `NativeType` 注册 +
  `NativeObject` impl，支持字段 getter、alias、skip、name override。
- **Varargs**:Java 的 `Method.isVarArgs()` + 参数打包由 Rust 闭包切片
  天然替代——注册的闭包接收 `&[DataValue]`，可自行处理变长参数。
- **数值提升**:BigInteger(任意精度)/BigDecimal(十进制字符串)的双向转换
  在构造器/方法闭包内部完成，无需修改 `ParametersTypeConvertor::cast`。
- **try/catch 控制信号**:对齐 Java `shouldExitTryCatch` 语义——
  仅传播 `Return` / `Break` / `Continue(null)`（循环控制哨兵），
  `Continue(non-null)` 作为块表达式结果留在栈上。

## 测试覆盖

```
cargo test --workspace --all-features: 777 passed / 0 failed / 0 ignored
```

| 类别 | 数量 | 说明 |
|---|---|---|
| 模块单元测试 | 270 | parser/runtime/operator/security 等 |
| Java 对齐测试 (`alignment_*`) | 338 | 对齐 Java 测试与官方脚本语义 |
| Rust 独立测试 (`rust_native_*`) | 62 | sandbox/property/error码/perf |
| Stage 集成测试 | 107 | QVM、编译、上下文、缓存、runner 与 derive |

### 验收门禁

| 门禁 | 结果 |
|---|---|
| 全量测试 | 777 passed / 0 failed / 0 ignored |
| 格式 | `cargo fmt --all -- --check` 通过 |
| 静态检查 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过 |
| 完整性 | 无 `todo!` / `unimplemented!` / `#[ignore]` / `compat.rs` |

## 构建 / 测试

```bash
cargo build --workspace
cargo test --workspace          # 全量:lib 单测 + tests/ 各 Stage 端到端
cargo clippy --workspace --all-targets
cargo doc --workspace --no-deps # 文档生成
```

## 生产验收

仓库提供 Java/Rust 自动差分、Java 官方脚本回放、线程独立 Runner 并发验收、
负载测试、安全 fuzz、业务宿主集成以及本地灰度/回滚演练。完整命令、门限与
真实生产环境边界见 [`docs/生产验收.md`](docs/生产验收.md)。

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
