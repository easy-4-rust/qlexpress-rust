# qlexpress-derive

`qlexpress` 的过程宏实现 crate，提供 `QLExpressType` 派生宏，将 Rust
宿主类型转换为 QLExpress 可注册的 `NativeType` 与 `NativeObject` 实现。

应用通常只需要依赖 `qlexpress`，由核心 crate 重新导出该宏：

```rust
use qlexpress::QLExpressType;

#[derive(QLExpressType)]
struct Order {
    amount: i64,
}
```

版本与 `qlexpress` 严格同步。完整使用说明见
[qlexpress](https://github.com/easy-4-rust/qlexpress-rust)。
