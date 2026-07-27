# QlExpress Rust

Alibaba QLExpress4 动态脚本引擎的 Rust 语义迁移实现。

当前 `0.1.0-alpha.1` 对齐 Java `4.2.0-beta`，提供表达式解析、QVM 执行、
宿主函数与类型注册、安全策略、编译缓存以及表达式追踪能力。

```toml
[dependencies]
qlexpress = "=0.1.0-alpha.1"
```

```rust
use qlexpress::{DataValue, Express4Runner, QLOptions};

let runner = Express4Runner::new();
let result = runner.execute(
    "19 + 23",
    Default::default(),
    &QLOptions::builder().build(),
)?;
assert_eq!(result.into_result(), DataValue::Int(42));
# Ok::<(), qlexpress::QLException>(())
```

完整文档、Java/Rust 对照关系及生产验收结果见
[GitHub 仓库](https://github.com/easy-4-rust/qlexpress-rust)。
