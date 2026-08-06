# 差分语料说明

`differential.jsonl` 是 CI 的共享 Java/Rust 可执行差分语料。每个用例必须能由固定 Java
基线和 Rust 执行器完整执行，比较标准化值、错误码、位置和 trace 数量。

`java-trace-baseline-failures.jsonl` 不是通过语料，也不是对 Rust 行为的豁免。它记录固定
Java `4.2.0-beta` 的可复现基线缺陷：`java-4.2.0-beta-trace-map-literal-npe` 在启用表达式
trace 时于 Java `TraceExpressionVisitor` 内部抛出 `NullPointerException`。该用例保留原始
脚本和故障原因，待上游基线修复或项目明确决定兼容此缺陷后，才能重新进入共享差分语料。

当前已验证 `differential.jsonl` 的 295 个用例 Java/Rust 完全一致。其中普通记录以
`script` 执行完整语言链路；`number_math` 记录以显式 Java `Number` 子类型直接调用
Java oracle。后者可选 `implementation`：未指定时调用 `NumberMath` 门面，指定
`IntegerMath`、`LongMath`、`BigIntegerMath`、`BigDecimalMath` 或
`FloatingPointMath` 时逐方法调用具体公开 override；`operator_manager` 记录则直接
比较注册、替换、查找、别名、优先级和适配器执行；`delegate_context` 记录直接构造
`DelegateQContext`，比较运行时委托、符号/函数/栈/作用域副作用以及异常路径；
`fixed_size_stack` 记录直接比较栈顺序、参数容器、显式/越界 null 与 Java
`StackSwapParameters` 对原栈数组的实时窗口副作用；`runtime_core` 记录直接
比较 `QRuntime`/`QvmRuntime`/`QContext` 的构造、委托、对象身份与附件双向写穿；
`exception_table` 记录直接比较 catch 声明顺序、Java 类型可赋值关系与可空 finally 位置；
`batch_add_function_result` 记录直接比较成功/失败列表的顺序、隔离性与可变写回；
`ql_functional_varargs` 记录直接比较空参数、异构参数顺序、null 参数和 null 返回值；
`lsp_position` 记录直接比较零基坐标、超长列和负数原样保存语义；`lsp_range`
与 `lsp_diagnostic` 记录直接比较正常字段以及 Java 可空引用的原样返回语义。

Java `OperatorManager.precedence("missing")` 当前实现抛 `NullPointerException`；
Rust 已保留同一可观察失败语义，并将该用例迁入 `differential.jsonl`。
`operator-manager-java-contract-divergences.jsonl` 只保留历史分类说明，不再含失败用例。
上述 trace 基线缺陷仍不得计入“已实现 Java 语义”的证据。

Java `src/test/resources/testsuite/**/*.ql` 的完整内容副本位于
`crates/qlexpress/tests/fixtures/java-testsuite/`。使用下列命令校验路径、数量和内容
均与 Java 基线一致。因仓库补丁格式不能保留“仅缺少末尾 LF”的文件状态，校验会单独报告
这类 EOF 规范化，不将其误报为脚本内容差异：

```bash
python3 verification/verify_java_testsuite_fixtures.py \
  --java-repo /path/to/QLExpress
```
