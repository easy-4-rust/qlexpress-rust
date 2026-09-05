# API 稳定性承诺

本文档记录 `qlexpress` crate 的公共 API 表面、稳定性分类与版本演进策略。

> 对应版本：`0.1.0`（首个正式版，SemVer 承诺已生效）
> 最后更新：2026-09-05

---

## 1. 稳定性承诺

### 1.1 SemVer 策略

`qlexpress` 遵循 [Semantic Versioning 2.0.0](https://semver.org/)：

| 版本阶段 | 承诺 |
|---------|------|
| `0.x.y-beta.*` / `0.x.y-alpha.*` | （历史通道）**不承诺**跨预发布的 API 兼容性。breaking change 可在任意预发布之间发生，但会在 CHANGELOG 中明确记录。 |
| `0.1.0`（正式版） | 首个稳定版本；minor/patch 升级遵循 SemVer，breaking change 需要 major 升级。 |
| `1.0.0` 及以后 | 严格 SemVer：minor 版本升级不破坏公共 API；breaking change 需要 major 版本升级。 |

### 1.2 公共 API 的定义

以下内容属于公共 API，受 SemVer 保护：

1. `lib.rs` 中所有 `pub use` re-export 的类型、trait、函数和宏。
2. `lib.rs` 中所有 `pub mod` 声明的模块路径。
3. 上述类型上所有 `pub` 方法、字段和关联函数（不含 `#[doc(hidden)]`）。
4. 派生宏 `QLExpressType` 的输入/输出契约。

以下内容**不属于**公共 API，不保证跨版本稳定：

1. `pub(crate)` 和 `pub(super)` 可见性的项目。
2. `#[doc(hidden)]` 标记的项目。
3. 模块内部的文件组织结构。
4. 测试辅助函数和 `#[cfg(test)]` 代码。

---

## 2. `lib.rs` Facade 公共 API 表

### 2.1 顶层 Re-exports

| 符号 | 来源模块 | 简介 | 稳定性 | 1.0 变动计划 |
|------|---------|------|--------|-------------|
| `CheckOptions` | `check_options` | 静态检查选项（操作符策略、函数调用开关） | stable | 无 |
| `ClassSupplier` | `class_supplier` | 宿主类型供应器 trait | stable | 无 |
| `DefaultClassSupplier` | `default_class_supplier` | 默认类型供应器实现 | stable | 无 |
| `Express4Runner` | `express4_runner` | 引擎门面：解析、编译、执行、函数/操作符注册 | stable | 无 |
| `InitOptions` | `init_options` | Runner 初始化选项（安全策略、调试、导入等） | stable | 无 |
| `QLOptions` | `ql_options` | 单次执行选项（缓存、超时、trace、附件等） | stable | 无 |
| `QLOptionsBuilder` | `ql_options` | QLOptions 构建器 | stable | 无 |
| `QLResult` | `ql_result` | 脚本执行结果（值 + 可选表达式 trace） | stable | 无 |
| `QLExpressType` | `qlexpress_derive` | 为宿主类型生成 NativeType/NativeObject 实现的派生宏 | stable | 无 |

### 2.2 异常体系 Re-exports（`exception`）

| 符号 | 简介 | 稳定性 | 1.0 变动计划 |
|------|------|--------|-------------|
| `ErrorReporter` | 错误报告器 trait | stable | 无 |
| `PureErrReporter` | 纯函数式错误报告器实现 | stable | 无 |
| `QLException` | 统一异常类型（携带完整诊断信息） | stable | 无 |
| `QLExceptionKind` | 异常类别枚举 | stable | 可能增加变体（`#[non_exhaustive]`） |
| `QLSyntaxException` | 语法异常（解析/编译阶段） | stable | 无 |

### 2.3 值与上下文 Re-exports（`runtime`）

| 符号 | 简介 | 稳定性 | 1.0 变动计划 |
|------|------|--------|-------------|
| `ExpressContext` | 外部变量上下文 trait | stable | 无 |
| `MapExpressContext` | HashMap 上下文实现 | stable | 无 |
| `QLAliasContext` | 别名对象上下文实现 | stable | 无 |
| `DataValue` | 脚本数据值枚举（Null/Bool/Int/Long/.../Object） | stable | 可能增加变体（`#[non_exhaustive]`） |
| `NativeObject` | 宿主对象 trait | stable | 无 |
| `QValue` | 脚本值引用类型 | stable | 无 |
| `Value` | 脚本值接口 trait | stable | 无 |

### 2.4 宿主扩展 Re-exports（`api` / `annotation` / `runtime`）

| 符号 | 简介 | 稳定性 | 1.0 变动计划 |
|------|------|--------|-------------|
| `QLFunctionMethod` | 函数方法元数据 | stable | 无 |
| `QLFunctionProvider` | 函数提供者 trait | stable | 无 |
| `BatchAddFunctionResult` | 批量注册函数结果 | stable | 无 |
| `QLFunctionalVarargs` | 变参函数 trait | stable | 无 |
| `CustomFunction` | 自定义函数 trait | stable | 无 |
| `CustomBinaryOperator` | 自定义二元操作符 trait | stable | 无 |

### 2.5 安全策略 Re-exports（`security`）

| 符号 | 简介 | 稳定性 | 1.0 变动计划 |
|------|------|--------|-------------|
| `CacheStats` | 编译缓存统计快照 | stable | 无 |
| `CancellationToken` | 跨线程协作式取消令牌 | stable | 无 |
| `Capability` | 宿主能力标识枚举 | stable | 可能增加变体（`#[non_exhaustive]`） |
| `CapabilityPolicy` | 能力白名单策略 | stable | 无 |
| `CompileCachePolicy` | 安全编译缓存策略 | stable | 无 |
| `NativeMember` | 原生成员描述符（类型名 + 成员名） | stable | 无 |
| `QLSecurityStrategy` | 安全策略枚举 | stable | 可能增加变体 |
| `ResourceLimits` | 资源预算配置 | stable | 可能增加字段 |
| `SandboxProfile` | 安全执行配置聚合 | stable | 可能增加字段 |

### 2.6 解析器/AST Re-exports（`aparser`）

| 符号 | 简介 | 稳定性 | 1.0 变动计划 |
|------|------|--------|-------------|
| `build_tree` | 解析脚本为语法树 | stable | 无 |
| `QLParser` | 递归下降解析器 | stable | 无 |
| `Node` | 语法树节点 | stable | 可能增加方法 |
| `TerminalNode` | 终结符节点 | stable | 无 |
| `Token` | 词法单元 | stable | 可能增加方法 |
| `HasChildren` | AST 节点子节点访问 trait | stable | 无 |
| `ChildRef` | 子节点引用枚举 | stable | 无 |
| `Visitor` | 解析器基础访问者 trait | stable | 无 |
| `CheckVisitor` | 静态检查访问者 | stable | 无 |
| `OutFunctionVisitor` | 函数名收集访问者 | stable | 无 |
| `OutVarAttrsVisitor` | 变量属性收集访问者 | stable | 无 |
| `OutVarNamesVisitor` | 变量名收集访问者 | stable | 无 |
| `ImportManager` | 导入管理器 | stable | 无 |
| `MacroDefine` | 宏定义 | stable | 无 |
| `CompileCache` | **unstable** -- 泛型编译缓存容器 | **unstable** | 后续版本可能移除或重命名（会先经 deprecated 过渡）；用户应使用 `SerializableParseCache` |
| `QCompileCache` | **unstable** -- 泛型编译缓存值 | **unstable** | 后续版本可能移除或重命名（会先经 deprecated 过渡）；用户应使用 `LoadedCompileCache` |
| `GeneratorScope` | **unstable** -- 编译期生成器作用域 | **unstable** | 内部实现细节，后续版本可能从 facade 移除 |

### 2.7 派生宏

| 符号 | 简介 | 稳定性 | 1.0 变动计划 |
|------|------|--------|-------------|
| `QLExpressType` | 为宿主类型生成 `NativeType` + `NativeObject` 实现 | stable | 属性参数可能扩展 |

---

## 3. 公开模块路径

`lib.rs` 声明了以下 `pub mod`，均可通过 `qlexpress::<module>` 访问：

| 模块 | 简介 | 稳定性 |
|------|------|--------|
| `annotation` | 函数/别名注解元数据 | stable |
| `aparser` | 词法/语法分析器、AST、编译缓存 | stable（内部子模块路径可能变动） |
| `api` | 公共 API 类型（BatchAddFunctionResult 等） | stable |
| `arithmetic` | 算术运算符 | stable |
| `assign` | 赋值运算符 | stable |
| `base` | 基础运算符 | stable |
| `bit` | 位运算符 | stable |
| `check_options` | 静态检查选项 | stable |
| `class_supplier` | 类型供应器 trait | stable |
| `collection` | 集合操作符（in/not_in） | stable |
| `compare` | 比较运算符 | stable |
| `compiletimefunction` | 编译期函数 | stable |
| `context` | 外部变量上下文 | stable |
| `convert` | 类型转换 | stable |
| `data` | 数据结构（DataValue、IndexMap、JavaArray 等） | stable |
| `default_class_supplier` | 默认类型供应器 | stable |
| `enums` | 枚举类型 | stable |
| `exception` | 异常体系 | stable |
| `express4_runner` | 引擎门面 | stable |
| `function` | 自定义函数体系 | stable |
| `init_options` | 初始化选项 | stable |
| `instruction` | QVM 指令集 | stable |
| `lambda` | Lambda 表达式 | stable |
| `logic` | 逻辑运算符 | stable |
| `lsp` | LSP 辅助类型（Position、Range、Diagnostic） | stable |
| `member` | 原生成员访问 | stable |
| `number` | 数值处理 | stable |
| `operator` | 操作符体系 | stable |
| `parsecache` | 可序列化 parse cache | stable |
| `proxy` | Lambda 代理 | stable |
| `ql_options` | 执行选项 | stable |
| `ql_precedences` | 操作符优先级常量 | stable |
| `ql_result` | 执行结果 | stable |
| `runtime` | 运行时模型（值、上下文、QVM、scope 等） | stable |
| `scope` | 作用域 | stable |
| `security` | 安全策略 | stable |
| `string` | 字符串操作符 | stable |
| `trace` | 表达式追踪 | stable |
| `unary` | 一元运算符 | stable |
| `util` | 工具函数 | stable（内部路径可能变动） |
| `utils` | 工具函数（Java 兼容命名） | stable（内部路径可能变动） |

> 注：`observability` 模块为 `pub(crate)`，不在公共 API 表面内。

---

## 4. 已废弃 API（Deprecated）

### 4.1 `Express4Runner::execute_with_alias_values`

- **废弃版本**：`0.1.0-beta.1`
- **替代方案**：使用 `Express4Runner::execute_with_alias_objects`
- **移除计划**：1.0.0
- **原因**：该方法是早期 Rust API 的兼容别名，命名与 Java `executeWithAliasObjects` 不一致。`execute_with_alias_objects` 保持与 Java 的对象名称一致性。

### 4.2 `CompileCache` / `QCompileCache`（unstable）

- **状态**：unstable，后续版本可能从 facade 移除或重命名（会先经 deprecated 过渡）
- **替代方案**：用户应使用 `api::parsecache::SerializableParseCache`（可序列化）和 `api::parsecache::LoadedParseCache`（已加载）
- **原因**：`CompileCache<L, T>` / `QCompileCache<L, T>` 是解析器内部的泛型缓存容器，暴露了不应由用户直接操作的实现细节。`Express4Runner` 内部使用 `CompileCacheStore` 管理缓存，用户通过 `export_parse_cache` / `import_parse_cache` / `set_parse_cache` 等方法操作缓存。

### 4.3 `GeneratorScope`（unstable）

- **状态**：unstable，后续版本可能从 facade 移除（会先经 deprecated 过渡）
- **原因**：编译期生成器作用域是解析器内部实现细节，不应暴露给用户。

---

## 5. 当前标记为 unstable 的项

以下项在后续 minor 版本仍可能调整（调整前会先以 deprecated 过渡并在 CHANGELOG 记录）：

| 项目 | 风险 | 说明 |
|------|------|------|
| `CompileCache` / `QCompileCache` re-export | 高 | 可能从 `lib.rs` facade 移除，仅通过 `aparser` 子模块访问 |
| `GeneratorScope` re-export | 高 | 可能从 `lib.rs` facade 移除 |
| `QLExceptionKind` 变体 | 低 | 可能增加新变体（已 `#[non_exhaustive]`） |
| `DataValue` 变体 | 低 | 可能增加新变体（已 `#[non_exhaustive]`） |
| `Capability` 变体 | 低 | 可能增加新变体（已 `#[non_exhaustive]`） |
| `ResourceLimits` 字段 | 低 | 可能增加新资源类型 |
| `SandboxProfile` 字段 | 低 | 可能增加新配置项 |
| `QLExpressType` 属性参数 | 低 | 可能增加新配置属性 |
| `lsp` 模块子类型 | 低 | 可能增加新诊断类型 |

---

## 6. 迁移指南：beta.0 到 1.0

### 6.1 `execute_with_alias_values` -> `execute_with_alias_objects`

```rust
// 旧写法（已废弃）
runner.execute_with_alias_values(script, &options, &[...])?;

// 新写法
runner.execute_with_alias_objects(script, &options, &[...])?;
```

方法签名完全相同，替换名称即可。

### 6.2 `CompileCache` / `QCompileCache` -> `SerializableParseCache`

```rust
// 旧写法（直接操作泛型缓存）
use qlexpress::CompileCache;
let mut cache = CompileCache::new();

// 新写法（通过 Runner API 操作缓存）
let cache = runner.export_parse_cache(script)?;
runner.set_parse_cache(&cache)?;
```

### 6.3 `DataValue` 变体穷尽性

如果 `match DataValue` 使用了通配符 `_` 以外的穷尽匹配，1.0 增加新变体时需要更新。建议始终保留 `_ =>` 分支：

```rust
match value {
    DataValue::Int(n) => { /* ... */ }
    DataValue::Str(s) => { /* ... */ }
    _ => { /* 处理其它类型 */ }
}
```

---

## 7. 如何报告 API 问题

如果在使用公共 API 时遇到以下情况，请通过 [GitHub Issues](https://github.com/easy-4-rust/qlexpress-rust/issues) 反馈：

- **API 设计缺陷**：方法命名不一致、参数顺序不合理、缺少必要的重载。
- **破坏性变更未记录**：升级版本后编译失败，但 CHANGELOG 中未记录 breaking change。
- **缺失的类型/方法**：某些操作需要访问 `pub(crate)` 内部类型才能完成。
- **文档不充分**：公共 API 缺少文档、示例或行为说明。

请使用以下 Issue 模板（如有）或在标题中标注 `[API]` 前缀，例如：

```
[API] Express4Runner::execute 缺少 &str 上下文的便捷重载
```

---

## 8. 变更日志

| 日期 | 变更 |
|------|------|
| 2026-09-05 | 初始版本：完成公共 API 审计、deprecated 标记、稳定性承诺文档 |
