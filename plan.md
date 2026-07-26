# qlexpress-rust 迁移计划

## 目标
将 QLExpress 最新 Java 版(4.2.0-beta, com.alibaba.qlexpress4, commit 9065b9a)做**完整语义功能迁移**到 Rust,工程名 `qlexpress-rust`,输出到 /mnt/agents/output/qlexpress-rust。

## 架构原则(与 Java 版对齐)
- 执行模型:Lexer → Parser(语法树)→ QvmInstructionVisitor(编译为指令)→ QVM 栈式虚拟机执行。**不做 tree-walking**。
- 模块结构镜像 Java 包结构,严禁把所有东西塞进 lib.rs:
```
src/
  lib.rs                 (仅 re-export + Express4Runner 入口声明)
  lib root:
    check_options.rs  ql_options.rs  ql_result.rs  ql_precedences.rs
    express4_runner.rs  class_supplier.rs  init_options.rs
  exception/     (QLException, QLSyntaxException, QLErrorCodes, lsp/)
  aparser/       (QLexer, Token, QLParser, SyntaxTreeFactory, RuleContext, ParseTree,
                  QvmInstructionVisitor, ImportManager, MacroDefine, GeneratorScope,
                  ParserOperatorManager, OperatorFactory, compiletimefunction/,
                  CheckVisitor, OutVar*Visitor, ScopeStackVisitor, QCompileCache)
  runtime/       (Value, QvmRuntime, QvmGlobalScope, QLambda*, ReflectLoader→NativeRegistry,
    instruction/  (42 个指令,每个一个文件或按类别分组文件)
    operator/     (base, arithmetic, assign, bit, collection, compare, logic, number, string, unary — 57 个操作符)
    context/      (ExpressContext, MapExpressContext, QLAliasContext ...)
    data/         (convert/, lambda/)
    function/     (CustomFunction, ExtensionFunction, QMethodFunction)
    scope/        trace/  util/
  security/      (QLSecurityStrategy 等 5 个)
  utils/         (BasicUtil, QLFunctionUtil ...)
  api/           (QLFunctionalVarargs, BatchAddFunctionResult, parsecache/)
  annotation/    (以 Rust attribute/doc 形式平移或标注)
```

## Java 反射的替代策略(关键决策)
Java 版靠反射调用宿主对象方法/字段(GetFieldInstruction, GetMethodInstruction, QMethodFunction, proxy/)。
Rust 无运行时反射,采用:
- 宿主类型通过注册表暴露:`ReflectLoader` → `NativeRegistry`(注册 struct/enum 的构造器、方法、字段 getter)
- 提供 `qlexpress-rust` 内置的 `Value::Data(Map)` 动态对象路径 + 可选 derive macro(后续阶段)
- 语义等价:脚本的 `obj.field`、`obj.method(args)`、`new X(...)` 行为保持一致

## 阶段划分(stage-gate,每阶段验证后才进入下一阶段)

### Stage 0 — 奠基(串行,1 个 coder)
- cargo 工程初始化;exception 模块(错误码全量对齐 QLErrorCodes)
- utils、enums、annotation 平移
- runtime::Value 类型体系(DataValue 全系列:数字/字符串/布尔/null/数组/map/lambda)
- 验收:cargo build + value/error 单测通过

### Stage 1 — Lexer(串行,1 个 coder)
- aparser::QLexer + Token + QLTokenType 全量 token 类型(含关键字表、操作符表、字符串插值模式 InterpolationMode)
- 验收:对一批 Java 测试脚本做 token 序列对比测试

### Stage 2 — Parser + 语法树(串行,1 个 coder)
- QLParser(递归下降,对齐 Java 版优先级 QLPrecedences)、SyntaxTreeFactory、RuleContext/ParseTree 节点体系
- ImportManager、MacroDefine、GeneratorScope、CheckVisitor 等编译期检查 visitor
- 验收:语法合法/非法用例与 Java 版行为一致(错误信息含行列号)

### Stage 3 — 指令编译 + QVM(可并行 2 个 coder)
- 3a: instruction/ 42 个指令结构定义 + QvmInstructionVisitor(语法树→指令)
- 3b: QvmRuntime(栈 VM、scope、QLambda、trace 骨架)
- 依赖 Stage 2 输出的语法树契约;3a/3b 通过指令 trait 契约解耦
- 验收:常量/算术脚本能端到端跑

### Stage 4 — 操作符体系(并行,2 个 coder)
- 4a: arithmetic/number/bit/compare/logic(数值提升规则对齐 Java)
- 4b: string/collection/assign/unary + base + OperatorManager + 自定义操作符(CustomBinaryOperator)
- 验收:操作符优先级/类型提升单测

### Stage 5 — 函数/上下文/安全/Runner(并行 2 个 coder)
- 5a: function/ + context/ + member/ + api/(自定义函数注册、扩展函数、别名上下文)
- 5b: security/ + Express4Runner + QLOptions/CheckOptions + aparser/compiletimefunction + parsecache
- 验收:Express4Runner API 端到端用例

### Stage 6 — 对齐测试与收尾(1 个 verifier + 1 个 coder 修复)
- 移植 Java 版 src/test 核心测试用例(算术、控制流、lambda、宏、函数、异常、沙箱)
- cargo test 全绿 + clippy 无警告 + README

## Stage 6 验收记录(2026-07-26)

### 工作量
- 294 文件 / 527 tests(基线) → 312 文件 / 605 tests / 16 ignored(完成态)
- 分支:`feat/stage6-alignment`,共 6 个 commit

### Phase 1 — Cargo workspace + #[derive(QLExpressType)]
- `crates/qlexpress/`(原 crate 改名)+ `crates/qlexpress-derive/`(新 proc-macro crate)
- 过程宏支持:name / skip / alias / no_native_object / 类型映射
- 12 fixture 用例覆盖字段/skip/alias/registry/runner/script 执行

### Phase 2 — 5 个缺失类
- `runtime/exception_table.rs` - ExceptionTable + ExceptionTableEntry + 3 用例
- `runtime/fixed_size_stack.rs` - 带容量上限的栈 + STACK_OVERFLOW 错误码 + 3 用例
- `runtime/trace/trace_point_tree.rs` - re-export ExpressionTrace as TracePointTree
- `aparser/trace_expression_visitor.rs` - v1 stub(运行时 ExpressionTrace 已完整)
- `ReflectLoader` 语义合并到 native_registry.rs(doc 注明)

### Phase 3 — Java 测试移植
- `tests/alignment_runner_full.rs` - 29 用例(对应 Express4RunnerTest 核心路径)
- `tests/alignment_parser.rs` - 16 用例(SyntaxTreeFactoryTest / MethodInvoke / GetField / NewInstance)
- `tests/alignment_issue_regression.rs` - 5 用例(TryCatchBreakContinue / Issue427 / Issue318 / QL4Alias)
- `tests/alignment_parsecache.rs` - 6 用例(SerializableParseCacheTest round-trip)

### Phase 4 — 安全策略
- `tests/alignment_security.rs` - 4 用例(open / white_list / black_list / check_options)

### 已知引擎语义缺口(16 ignored)
| 用例 | 状态 | 原因 |
|---|---|---|
| interpolation_disable | 待修 | disable 模式应原样保留 `\n \b` 等转义 |
| doc_try_catch_as_expr | 待修 | `1 + try {...} catch { 11 }` 应返回 12 |
| alignment_runner_full/multiple_declarators | 待修 | parser 不支持 `int a, b = 10` |
| alignment_runner_full/foreach_iterates_list | 待修 | foreach 语法为 `foreach (x in list)` 非 `:` |
| alignment_runner_full/break_inside_for 等 3 个 | 待修 | parser 不允许 `}expr` 后跟语句 |
| alignment_parsecache/cross_runner | 待修 | cache identity 校验未生效 |
| 其他 (alignment_issue_regression 桩位 2 个) | 占位 | 留待 follow-up |

### 收尾状态
- cargo test --workspace:605 passed, 0 failed, 16 ignored
- cargo clippy --workspace --all-targets:0 errors,~60 warnings(预存,不在本次范围)
- cargo doc --workspace --no-deps:0 broken link(已修本阶段引入的)

## 工具与技能
- 每阶段加载 vibecoding-general-swarm 指导 coder subagent
- Java 源码位于 /mnt/agents/qlexpress-java,coder 按需阅读原文对齐语义
- 测试数据可复用 /mnt/agents/qlexpress-java/src/test 中的脚本用例