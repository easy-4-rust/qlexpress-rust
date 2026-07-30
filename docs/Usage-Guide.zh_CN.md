# QlExpress Rust 使用指南

> **文档目的**：提供从首次执行到安全宿主集成的源码可追溯路径。<br>
> **适用版本**：`qlexpress 0.1.0-alpha.2`<br>
> **Rust 基线**：MSRV 1.85，Edition 2021<br>
> **最后核验**：2026-07-27<br>
> **状态**：Alpha 文档；`1.0` 之前 API 仍可能变化

[English](Usage-Guide.md) | [项目 README](../README.zh-CN.md) |
[架构文档](qlexpress-Architecture.zh_CN.md)

## 1. 心智模型

应用持有 `Express4Runner`，注册脚本可以使用的宿主能力，再使用上下文和单次执行
`QLOptions` 运行脚本。

```text
一次性配置 runner
  → 注册函数/操作符/类型
  → 校验或预编译脚本
  → execute(script, context, options)
  → 读取 QLResult 或 QLException
```

三类选项的生命周期不同：

| 类型 | 生命周期 | 示例 |
|:---|:---|:---|
| `InitOptions` | runner 构造期 | 安全策略、追踪支持、插值、调试 |
| `QLOptions` | 单次执行策略，可复用 | 超时、缓存、附件、数组限制、追踪 |
| `CheckOptions` | 静态校验 | 操作符白/黑名单、禁止函数调用 |

## 2. 安装与运行

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

在本仓库中运行：

```bash
cargo run -p qlexpress --example quick_start
```

预期输出：

```text
100.0
```

## 3. 脚本与值

代表性语法：

```text
// 算术与条件
score = base + bonus;
score >= 80 ? 'PASS' : 'REVIEW'

// 列表、Map 与字段访问
items = [1, 2, 3];
result = {'count': items.size(), 'first': items[0]};

// Lambda 与函数
twice = x -> x * 2;
function add(a, b) { return a + b; }
add(twice(10), 22)

// 控制流
sum = 0;
for (i = 1; i <= 4; i = i + 1) { sum = sum + i; }
sum
```

宿主边界统一使用 `DataValue`：

| Rust 变体 | 脚本/Java 风格含义 |
|:---|:---|
| `Null`、`Bool`、`Char`、`Str` | null、布尔、字符、字符串 |
| `Byte`、`Short`、`Int`、`Long` | 整数 |
| `Float`、`Double`、`BigInt`、`BigDec` | 浮点与任意精度数值 |
| `List`、`Array`、`Map` | 具有引用语义的可变集合 |
| `Lambda` | 已编译脚本 Lambda |
| `Object` | 显式注册的宿主对象 |

集合和宿主对象内部使用 `Rc<RefCell<...>>`，在单个 runner 线程内保留 Java 风格的
引用语义。

## 4. 执行选项

```rust
let options = QLOptions::builder()
    .cache(true)
    .timeout_millis(500)
    .max_arr_length(10_000)
    .avoid_null_pointer(true)
    .build();
```

| 选项 | 默认值 | 作用 |
|:---|:---:|:---|
| `precise` | `false` | 在支持的数值路径启用精确十进制计算 |
| `pollute_user_context` | `false` | 允许脚本全局变量回写宿主上下文 |
| `timeout_millis` | `-1` | `<= 0` 表示引擎不限制超时 |
| `attachments` | 空 | 仅供宿主扩展函数读取的附加数据 |
| `cache` | `false` | 相同脚本文本复用编译产物 |
| `avoid_null_pointer` | `false` | 启用 Java 对应的空指针规避行为 |
| `max_arr_length` | `-1` | 限制脚本创建的数组长度 |
| `trace_expression` | `false` | `InitOptions` 同时启用时采集表达式追踪 |
| `short_circuit_disable` | `false` | 禁用逻辑短路 |

引擎超时由编译器插入的 QVM 指令协作检查。宿主仍应按威胁模型设置请求 deadline、
输入限制和进程级隔离。

## 5. 自定义函数

函数需要执行上下文与类型化参数时使用闭包：

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

简单变参函数使用 `add_varargs_function`：

```rust
runner.add_varargs_function("sumAll", |params: &[DataValue]| {
    let sum = params.iter().fold(0, |sum, value| match value {
        DataValue::Int(value) => sum + value,
        _ => sum,
    });
    Ok(DataValue::Int(sum))
});
```

注册采用 `putIfAbsent` 语义：同名函数已存在时返回 `false`。
`batch_add_function` 会分别返回成功和失败的名称。

## 6. 自定义操作符与别名

自定义操作符会改变 runner 配置，因此要求 `&mut Express4Runner`：

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

除非迁移兼容确有需要，否则不建议替换内建操作符。替换会改变该 runner 执行的所有
脚本含义。

## 7. 使用 `QLExpressType` 暴露宿主结构体

派生宏为具名字段、非泛型结构体生成原生类型元数据和字段访问：

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

支持的辅助属性：

| 位置 | 属性 | 含义 |
|:---|:---|:---|
| 结构体 | `name = "..."` | 覆盖注册类型名 |
| 结构体 | `expose_fields` | 同时通过方法式解析暴露字段 |
| 字段 | `skip` | 不暴露该字段 |
| 字段 | `readonly` | 仅生成读取能力；脚本赋值和分类对象填充会拒绝写入 |
| 字段 | `alias("a", "b")` | 添加字段别名 |

当前限制：仅支持具名字段结构体，不支持泛型结构体，也不会自动发现方法和构造器。
后两者应通过 `NativeRegistry` 或 runner 的显式 API 注册。

## 8. 安全与静态校验

不可信输入必须使用 `Express4Runner::execute_checked` 和
`SandboxProfile::secure()`。普通 `execute` 有意保留 Java 兼容的无限默认值。安全入口把
静态校验、解析/编译/运行有限预算、capability 白名单、取消和租户有界 LRU 组合成一条
不可绕过路径。敌对输入还应通过可选的 `qlexpress-process` 隔离进程执行器运行，详见
[安全沙箱](Security-Sandbox.zh_CN.md)。

默认原生成员策略是隔离：

```rust
let init = InitOptions::builder()
    .security_strategy(QLSecurityStrategy::isolation())
    .build();
let runner = Express4Runner::with_init_options(init);
```

对脚本必须访问的少量宿主成员使用 `QLSecurityStrategy::white_list`。`open()` 只适合
可信输入或受严格控制的示例。

执行前可通过静态校验限制语法：

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

静态校验本身不能保证任意脚本安全，必须与成员策略、资源限制、超时、fuzz 和部署
隔离配合。

## 9. 编译缓存与可移植 Parse Cache

`QLOptions::cache(true)` 按完整脚本文本在 runner 内缓存编译产物。缓存仅属于该
runner 和线程。

可序列化缓存的基本流程：

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

已加载缓存绑定到执行导入的 runner。可用 `set_parse_cache` 预热普通编译缓存。
缓存 JSON 是版本化数据，应校验模型版本、生产者版本、脚本哈希和可信来源。

## 10. 表达式追踪与静态分析

runner 初始化和单次执行都必须开启追踪：

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

静态分析 API：

- `get_out_var_names`：宿主需要提供的变量；
- `get_out_var_attrs`：访问的属性路径；
- `get_out_function_names`：引用的外部函数；
- `get_expression_trace_points`：不执行脚本获取表达式追踪树；
- `parse_to_syntax_tree`、`parse_to_instructions`：诊断和工具支持。

## 11. 错误处理

所有执行失败都使用 `QLException`。应读取稳定字段，而不是解析格式化消息：

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

`QLExceptionKind` 区分语法、运行时和超时错误；`check` 返回更窄的
`QLSyntaxException`。

## 12. 并发模型

不要在线程间共享同一个 runner。每个 worker 内创建并配置一个 runner：

```text
请求调度器
  ├── worker 1 → runner 1 + cache 1
  ├── worker 2 → runner 2 + cache 2
  └── worker N → runner N + cache N
```

仓库验收命令：

```bash
cargo run -p qlexpress-verification -- concurrency 8 2000
```

## 13. 生产接入检查单

- 固定 crate 版本；迁移项目同时记录 Java 兼容基线。
- 盘点并评审暴露的每个函数、操作符、类型、字段、方法和构造器。
- 使用隔离或白名单；不可信脚本禁止默认使用 `open()`。
- 设置脚本、上下文、超时、数组、内存和请求并发限制。
- 切流前回放真实脱敏脚本并对比结果。
- 在生产相近硬件和真实 worker 模型下执行负载与 soak 测试。
- 在宿主监控中记录错误码、延迟、缓存行为和业务决策。
- 使用稳定决策链对候选引擎做灰度比对，并定义自动回滚条件。

仓库命令和已记录结果见[生产验收](生产验收.md)。它们不能替代业务宿主自己的
staging 与 canary 证据。

---

**文档版本**：1.0.0<br>
**创建日期**：2026-07-27<br>
**最后更新**：2026-07-27<br>
**文档状态**：待评审
