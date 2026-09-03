# RUST_EXTENSION 方法审计报告

- **审计日期**：2026-09-03
- **审计对象**：`verification/migration-manifest-current.json` 中 `state == "RUST_EXTENSION"` 的全部 13 个方法条目（生成时间 2026-08-09T18:24:22+00:00）
- **审计基准**：Java QLExpress4 `4.2.0-beta` @ `9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3`
- **审计人**：人工复核（生成器 `generate_migration_manifest.py` 的自动标记逐一核对源码证据）

## 一、结论（TL;DR）

**13 个 RUST_EXTENSION 全部为生成器"名字匹配盲区"造成的误标，不存在缺少 Java 行为对照的真实 Rust 扩展。**

- 类别 A（刻意 Rust 扩展，无 Java 对应，需补差分场景）：**0 个**
- 类别 B（Java 有对应行为但迁移遗漏）：**0 个**
- 类别 C（Java 行为已实现，因 Rust 语言设施/架构替代导致生成器无名字可匹配）：**13 个**

业务脚本影响评估：这 13 个方法全部是**引擎内部实现面**（编译器访客方法、值语义 equals/hashCode、反射缓存键访问器），**不在脚本可调用 API 表面**，业务脚本不会直接依赖；差分语料（295 条）与回放（151 条）已间接覆盖其行为。

## 二、逐条审计表

### 组 1：编译器访客方法（2 个）— 适配名覆盖

| # | manifest 索引 | Java 方法 | Java 位置 | 判定 | Rust 实现锚点 |
|---|---|---|---|---|---|
| 1 | 644 | `QvmInstructionVisitor::visitArrayInitializer` | QvmInstructionVisitor.java:1664 | C：适配名 | `crates/qlexpress/src/aparser/qvm_instruction_visitor/compilation_helpers.rs:402-422`（`new_arr_with_initializers`，Java 访客方法在 Rust 并入编译辅助函数）；`visit_expressions.rs:283`（调用点）；`syntax_tree_factory/node_dispatch.rs:31`（`ArrayInitializer` 分派） |
| 2 | 645 | `QvmInstructionVisitor::visitConstExpr` | QvmInstructionVisitor.java:1669 | C：适配名 | 访客 trait `crates/qlexpress/src/aparser/ql_parser_base_visitor.rs:130`（`visit_const_expr`）；分派 `parse_tree.rs:295`；trace 访客 `trace_expression_visitor.rs:462-463`（rustdoc 明确"对应 Java 方法 `visitConstExpr`"）；常量指令编译在 `qvm_instruction_visitor/visit_expressions.rs` 字面量路径（对应 Java 读取 literal 上下文并发射 `ConstInstruction`） |

生成器盲区原因：Java 侧是 ANTLR 访客回调方法，Rust 侧编译逻辑按语义域拆入辅助函数/访客 trait，函数名与 Java 不一一对应，名字匹配无候选。

### 组 2：MetaClass 值语义（2 个）— 语言设施替代

| # | manifest 索引 | Java 方法 | Java 位置 | 判定 | Rust 实现锚点 |
|---|---|---|---|---|---|
| 3 | 953 | `MetaClass::equals` | MetaClass.java:20 | C：derive 替代 | `crates/qlexpress/src/runtime/meta_class.rs:20-22`：`#[derive(Clone, Debug, PartialEq, Eq, Hash)]`，rustdoc 明确"`PartialEq/Eq/Hash` 全部仅按 `clz`，对应 Java `equals/hashCode`" |
| 4 | 954 | `MetaClass::hashCode` | MetaClass.java:30 | C：derive 替代 | 同上 |

生成器盲区原因：Java 手写 `equals/hashCode` 在 Rust 惯用写法是 `derive(PartialEq, Eq, Hash)`，无同名方法可匹配；语义（仅按 `clz` 判等/散列）已由 derive 语义等价承载并有注释锚定。

### 组 3：ReflectLoader 反射缓存键（8 个）— 架构替代

Java 侧 `ExtensionMapKey` 与 `MethodCacheKey` 是 JVM 反射发现的内部缓存键值类（getter + equals + hashCode）。Rust 侧在 `crates/qlexpress/src/runtime/reflect_loader.rs:25-32` 模块文档中**明确记录了该设计决策**：

> Java 的下列内部键/缓存对象只服务 JVM 反射发现；Rust 把已经解析的字段、方法和扩展函数直接存入 `NativeRegistry` 的类型化 `HashMap`，从而保留重用语义而不保留 JVM `Class`/`Method` 身份：
> - 对应 Java: `ReflectLoader.FieldReflectCache`
> - 对应 Java: `ReflectLoader.ExtensionMapKey`
> - 对应 Java: `ReflectLoader.MethodCacheKey`

| # | manifest 索引 | Java 方法 | Java 位置 | 判定 | Rust 实现锚点 |
|---|---|---|---|---|---|
| 5 | 1023 | `ExtensionMapKey::getCls` | ReflectLoader.java:395 | C：NativeRegistry 替代 | `reflect_loader.rs:25-32` 模块文档（上引）；扩展函数存于 `NativeRegistry` 类型化 HashMap |
| 6 | 1024 | `ExtensionMapKey::getMethodName` | ReflectLoader.java:399 | C：同上 | 同上 |
| 7 | 1025 | `ExtensionMapKey::equals` | ReflectLoader.java:403 | C：同上（HashMap 键语义由 `Eq`/`Hash` 承载） | 同上 |
| 8 | 1026 | `ExtensionMapKey::hashCode` | ReflectLoader.java:413 | C：同上 | 同上 |
| 9 | 1028 | `MethodCacheKey::getCls` | ReflectLoader.java:432 | C：同上 | 同上 |
| 10 | 1029 | `MethodCacheKey::getMethodName` | ReflectLoader.java:436 | C：同上 | 同上 |
| 11 | 1030 | `MethodCacheKey::getArgTypes` | ReflectLoader.java:440 | C：同上 | 同上 |
| 12 | 1031 | `MethodCacheKey::equals` | ReflectLoader.java:444 | C：同上 | 同上 |
| 13 | 1032 | `MethodCacheKey::hashCode` | ReflectLoader.java:454 | C：同上 | 同上 |

生成器盲区原因：键值类整体消失于"类型化 HashMap 直接存储"的架构决策（与"JVM 反射 → NativeRegistry 显式注册"同源），10 个访问器/值语义方法无逐一对应的 Rust 具名方法。

## 三、建议动作

1. **台账升级（下轮生成器运行时）**：按 manifest 的 `reviewed_disposition` 机制将 13 条升级为 `IMPLEMENTED`，逐条附本报告第二节锚点作为 `reviewed_test_evidence`/source anchor（符合 `migration-manifest-summary.json` policy 的 reviewed_disposition_rule：精确基线 + Java key + 语义理由 + Rust 源锚点 + 测试锚点）。不要手改 `migration-manifest-current.json`（机器生成物）。
2. **生成器改进（可选，长期）**：在 `generate_migration_manifest.py` 增补两类自动识别——(a) `derive(PartialEq, Eq, Hash)` + "对应 Java equals/hashCode" 注释 → 值语义方法；(b) 模块级设计决策文档（如 `reflect_loader.rs:25-32`）中列名的 Java 类型 → 其成员整体归入该决策处置。可将此类误标从 13 降到 0。
3. **业务脚本无需任何动作**：13 条均不在脚本 API 表面，无需补差分场景；现有 295 差分 + 151 回放 + 223 源测试映射已构成其行为证据。

## 四、复核清单

- [x] 13 条逐一打开 Java 源与 Rust 实现比对
- [x] 每条给出可点击的 Rust 锚点（file:line）
- [x] 确认无脚本可调用面受影响
- [x] 与 `docs/对象级对照表.md` / `docs/迁移差异审计基线.md` 的既有结论不冲突（组 3 与 NativeRegistry 架构决策同源）
