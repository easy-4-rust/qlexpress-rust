# QLExpress-Rust 合成业务语料 -- 错误码覆盖报告

> 生成时间: 2026-09-05T07:46:30.732040+00:00
> 错误码来源: `crates/qlexpress/src/exception/ql_error_codes.rs` (67 个)
> 仓库语料: `verification/corpus/differential.jsonl` (295 条, 覆盖 9 错误码)
> 合成语料: `verification/corpus/business-synthetic.jsonl` (76 条)

## 1. 总览

| 指标 | 值 |
|---|---|
| 错误码总数 | 67 |
| 仓库语料已覆盖 | 9 |
| 合成语料覆盖 | 67 |
| 合成语料中可通过脚本触发 | 67 |
| 合成语料中需要非脚本触发 | 9 |

## 2. 66 错误码 x 覆盖矩阵

| 错误码 | 严重度 | 仓库语料 | 合成语料条数 | 触发方式 |
|---|---|---|---|---|
| `ARRAY_SIZE_NUM_REQUIRED` | P1 | -- | 1 | script |
| `BIZ_EXCEPTION` | P1 | -- | 1 | non-script / manual |
| `CLASS_NOT_FOUND` | P0 | -- | 1 | script |
| `CONDITION_BOOL_REQUIRED` | P1 | -- | 1 | script |
| `EXCEED_MAX_ARR_LENGTH` | P1 | YES | 2 | script |
| `EXECUTE_BLOCK_ERROR` | P1 | -- | 1 | script |
| `EXECUTE_CATCH_HANDLER_ERROR` | P1 | -- | 1 | script |
| `EXECUTE_FINAL_BLOCK_ERROR` | P1 | -- | 1 | script |
| `EXECUTE_OPERATOR_EXCEPTION` | P1 | -- | 1 | script |
| `EXECUTE_TRY_BLOCK_ERROR` | P1 | -- | 1 | script |
| `FIELD_NOT_FOUND` | P1 | -- | 2 | script |
| `FOR_BODY_ERROR` | P1 | -- | 1 | script |
| `FOR_CONDITION_BOOL_REQUIRED` | P1 | -- | 1 | script |
| `FOR_CONDITION_ERROR` | P1 | -- | 1 | script |
| `FOR_EACH_ITERABLE_REQUIRED` | P1 | -- | 1 | script |
| `FOR_EACH_TYPE_MISMATCH` | P1 | -- | 1 | script |
| `FOR_EACH_UNKNOWN_ERROR` | P1 | -- | 1 | script |
| `FOR_INIT_ERROR` | P1 | -- | 1 | script |
| `FOR_UPDATE_ERROR` | P1 | -- | 1 | script |
| `FUNCTION_NOT_FOUND` | P1 | YES | 1 | script |
| `FUNCTION_TYPE_MISMATCH` | P1 | -- | 1 | script |
| `GET_FIELD_UNKNOWN_ERROR` | P1 | -- | 1 | script |
| `INCOMPATIBLE_ARRAY_ITEM_TYPE` | P1 | -- | 1 | script |
| `INCOMPATIBLE_ASSIGNMENT_TYPE` | P1 | -- | 1 | script |
| `INCOMPATIBLE_TYPE_CAST` | P1 | -- | 1 | script |
| `INDEX_OUT_BOUND` | P1 | YES | 1 | script |
| `INVALID_ARGUMENT` | P1 | -- | 1 | non-script / manual |
| `INVALID_ARITHMETIC` | P1 | YES | 3 | script |
| `INVALID_ASSIGNMENT` | P1 | YES | 2 | script |
| `INVALID_BINARY_OPERAND` | P1 | YES | 1 | script |
| `INVALID_CAST_TARGET` | P1 | -- | 1 | script |
| `INVALID_INDEX` | P1 | YES | 1 | script |
| `INVALID_NUMBER` | P0 | -- | 1 | script |
| `INVALID_UNARY_OPERAND` | P1 | -- | 1 | script |
| `INVOKE_CONSTRUCTOR_INNER_ERROR` | P1 | -- | 1 | script |
| `INVOKE_CONSTRUCTOR_UNKNOWN_ERROR` | P1 | -- | 1 | script |
| `INVOKE_FUNCTION_INNER_ERROR` | P1 | -- | 1 | script |
| `INVOKE_LAMBDA_ERROR` | P1 | -- | 1 | script |
| `INVOKE_METHOD_INNER_ERROR` | P1 | -- | 1 | script |
| `INVOKE_METHOD_UNKNOWN_ERROR` | P1 | -- | 1 | script |
| `INVOKE_METHOD_WITH_WRONG_ARGUMENTS` | P1 | -- | 1 | script |
| `METHOD_NOT_FOUND` | P1 | -- | 2 | script |
| `MISSING_INDEX` | P0 | -- | 1 | script |
| `NONINDEXABLE_OBJECT` | P1 | -- | 1 | script |
| `NONTRAVERSABLE_OBJECT` | P1 | -- | 1 | script |
| `NO_SUITABLE_CONSTRUCTOR` | P1 | -- | 1 | script |
| `NULL_CALL` | P1 | -- | 1 | script |
| `NULL_FIELD_ACCESS` | P1 | YES | 1 | script |
| `NULL_METHOD_ACCESS` | P1 | -- | 1 | script |
| `OBJECT_NOT_CALLABLE` | P1 | -- | 1 | script |
| `OPERAND_STACK_OVERFLOW` | P0 | -- | 1 | script |
| `OPERAND_STACK_UNDERFLOW` | P0 | -- | 1 | non-script / manual |
| `OPERATOR_NOT_ALLOWED` | P2 | -- | 1 | script |
| `PARSE_AST_DEPTH_EXCEEDED` | P0 | -- | 1 | script |
| `QL_THROW` | P1 | -- | 1 | script |
| `SCRIPT_TIME_OUT` | P1 | -- | 1 | script |
| `SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND` | P2 | -- | 1 | non-script / manual |
| `SERIALIZABLE_PARSE_CACHE_INVALID_MODEL` | P2 | -- | 1 | non-script / manual |
| `SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND` | P2 | -- | 1 | non-script / manual |
| `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT` | P2 | -- | 1 | non-script / manual |
| `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION` | P2 | -- | 1 | non-script / manual |
| `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION` | P2 | -- | 1 | non-script / manual |
| `SET_FIELD_UNKNOWN_ERROR` | P1 | -- | 1 | script |
| `STACK_OVERFLOW` | P0 | -- | 1 | script |
| `SYNTAX_ERROR` | P0 | YES | 4 | script |
| `WHILE_CONDITION_BOOL_REQUIRED` | P1 | -- | 1 | script |
| `WHILE_CONDITION_ERROR` | P1 | -- | 1 | script |

## 3. 仓库语料未覆盖的错误码 -- 构造方案

共 58 个错误码在仓库 295 条语料中未被触发：

### `ARRAY_SIZE_NUM_REQUIRED` (P1, array)

- **脚本**: `a = new int['not_a_number']`
  - 触发原因: Array size must be a number, got string

### `BIZ_EXCEPTION` (P1, user-exception)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- thrown by user-registered business logic

### `CLASS_NOT_FOUND` (P0, type-resolution)

- **脚本**: `obj = new com.example.NonExistentClass()`
  - 触发原因: Reference to non-existent class

### `CONDITION_BOOL_REQUIRED` (P1, control-flow)

- **脚本**: `if (42) { x = 1 }`
  - 触发原因: if condition must be boolean, got int

### `EXECUTE_BLOCK_ERROR` (P1, block-execution)

- **脚本**: `{ 1/0 }`
  - 触发原因: Block body throws arithmetic error

### `EXECUTE_CATCH_HANDLER_ERROR` (P1, try-catch)

- **脚本**: `try { 1/0 } catch(e) { 1/0 }`
  - 触发原因: catch handler itself throws error

### `EXECUTE_FINAL_BLOCK_ERROR` (P1, try-catch)

- **脚本**: `try { x = 1 } finally { 1/0 }`
  - 触发原因: finally block throws arithmetic error

### `EXECUTE_OPERATOR_EXCEPTION` (P1, operator)

- **脚本**: `1 / 0`
  - 触发原因: Division by zero triggers operator exception

### `EXECUTE_TRY_BLOCK_ERROR` (P1, try-catch)

- **脚本**: `try { 1/0 } catch(e) { 'caught' }`
  - 触发原因: try block throws error (should be caught)

### `FIELD_NOT_FOUND` (P1, field-access)

- **脚本**: `m = {'a': 1}; m.nonExistentField`
  - 触发原因: Access non-existent field on map
- **脚本**: `order = {'id':'O-1', 'amt':100}; order.amount`
  - 触发原因: Rule author uses dot on Map context expecting POJO field — QLExpress isolation strategy blocks reflective access to non-whitelisted fields

### `FOR_BODY_ERROR` (P1, control-flow)

- **脚本**: `for (i = 0; i < 10; i++) { 1/0 }`
  - 触发原因: for-body throws arithmetic error

### `FOR_CONDITION_BOOL_REQUIRED` (P1, control-flow)

- **脚本**: `for (i = 0; 42; i++) { i }`
  - 触发原因: for-condition must return boolean, got int

### `FOR_CONDITION_ERROR` (P1, control-flow)

- **脚本**: `for (i = 0; 1/0; i++) { i }`
  - 触发原因: for-condition throws arithmetic error

### `FOR_EACH_ITERABLE_REQUIRED` (P1, control-flow)

- **脚本**: `for (x : 42) { x }`
  - 触发原因: for-each requires iterable, got integer

### `FOR_EACH_TYPE_MISMATCH` (P1, control-flow)

- **脚本**: `for (int x : ['a','b']) { x }`
  - 触发原因: for-each expects int but got String

### `FOR_EACH_UNKNOWN_ERROR` (P1, control-flow)

- **脚本**: `for (x : null) { x }`
  - 触发原因: for-each on null causes unknown error

### `FOR_INIT_ERROR` (P1, control-flow)

- **脚本**: `for (1/0; i < 10; i++) { i }`
  - 触发原因: for-init expression throws arithmetic error

### `FOR_UPDATE_ERROR` (P1, control-flow)

- **脚本**: `for (i = 0; i < 10; i = 1/0) { i }`
  - 触发原因: for-update throws arithmetic error

### `FUNCTION_TYPE_MISMATCH` (P1, function-invocation)

- **脚本**: `x = 42; x()`
  - 触发原因: Variable is not a function type but is called

### `GET_FIELD_UNKNOWN_ERROR` (P1, field-access)

- **脚本**: `x = 1; x.unknownInternalField`
  - 触发原因: Attempt to get field that causes internal error

### `INCOMPATIBLE_ARRAY_ITEM_TYPE` (P1, array)

- **脚本**: `int[] a = [1, 2, 'three']`
  - 触发原因: Array declared as int[] but contains String

### `INCOMPATIBLE_ASSIGNMENT_TYPE` (P1, assignment)

- **脚本**: `int x = 'not a number'`
  - 触发原因: Assign string to int-typed variable

### `INCOMPATIBLE_TYPE_CAST` (P1, type-cast)

- **脚本**: `(Integer)'hello'`
  - 触发原因: Cannot cast String to Integer

### `INVALID_ARGUMENT` (P1, user-exception)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- thrown by user-registered functions with invalid args

### `INVALID_CAST_TARGET` (P1, type-cast)

- **脚本**: `(42) 'hello'`
  - 触发原因: Cast target is not a class/type

### `INVALID_NUMBER` (P0, syntax-parsing)

- **脚本**: `x = 12abc`
  - 触发原因: Malformed numeric literal

### `INVALID_UNARY_OPERAND` (P1, operator)

- **脚本**: `-'hello'`
  - 触发原因: Negation of String is not valid

### `INVOKE_CONSTRUCTOR_INNER_ERROR` (P1, constructor-invocation)

- **脚本**: `new String(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)`
  - 触发原因: Constructor with too many args causes inner error

### `INVOKE_CONSTRUCTOR_UNKNOWN_ERROR` (P1, constructor-invocation)

- **脚本**: `new Object()`
  - 触发原因: Constructor invocation may fail in restricted sandbox

### `INVOKE_FUNCTION_INNER_ERROR` (P1, function-invocation)

- **脚本**: `function boom() { return 1/0; }; boom()`
  - 触发原因: Function body throws arithmetic error

### `INVOKE_LAMBDA_ERROR` (P1, lambda-invocation)

- **脚本**: `f = x -> { return 1/0; }; f(1)`
  - 触发原因: Lambda body throws arithmetic error

### `INVOKE_METHOD_INNER_ERROR` (P1, method-invocation)

- **脚本**: `s = 'hello'; s.charAt(-1)`
  - 触发原因: charAt with negative index causes inner exception

### `INVOKE_METHOD_UNKNOWN_ERROR` (P1, method-invocation)

- **脚本**: `s = 'hello'; s.nonExistentMethod()`
  - 触发原因: Call method that does not exist on String

### `INVOKE_METHOD_WITH_WRONG_ARGUMENTS` (P1, method-invocation)

- **脚本**: `s = 'hello'; s.substring()`
  - 触发原因: substring() called with zero arguments

### `METHOD_NOT_FOUND` (P1, method-invocation)

- **脚本**: `obj = {'a':1}; obj.noSuchMethod(1,2,3)`
  - 触发原因: No suitable method 'noSuchMethod' on map
- **脚本**: `user = {'id':'U-1'}; user.getPassword()`
  - 触发原因: Production anti-pattern: script attempts privileged getter not in whiteList — isolation security blocks

### `MISSING_INDEX` (P0, syntax-parsing)

- **脚本**: `a = [1,2,3]; a[]`
  - 触发原因: Empty brackets -- missing index expression

### `NONINDEXABLE_OBJECT` (P1, index-access)

- **脚本**: `x = 42; x[0]`
  - 触发原因: Integer is not indexable

### `NONTRAVERSABLE_OBJECT` (P1, iteration)

- **脚本**: `for (x : 42) { x }`
  - 触发原因: Integer is not traversable in for-each

### `NO_SUITABLE_CONSTRUCTOR` (P1, constructor-invocation)

- **脚本**: `new Integer('not_a_number')`
  - 触发原因: No constructor matches argument type

### `NULL_CALL` (P1, null-safety)

- **脚本**: `f = null; f()`
  - 触发原因: Call null as function

### `NULL_METHOD_ACCESS` (P1, null-safety)

- **脚本**: `null.toString()`
  - 触发原因: Call method on null literal

### `OBJECT_NOT_CALLABLE` (P1, invocation)

- **脚本**: `obj = 42; obj()`
  - 触发原因: Integer is not callable

### `OPERAND_STACK_OVERFLOW` (P0, resource-limits)

- **脚本**: `(((((((((((((((((((((((((1+1))))))))))))))))))))))))`
  - 触发原因: Deeply nested expressions exhaust operand stack

### `OPERAND_STACK_UNDERFLOW` (P0, resource-limits)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Empty script -- no operands to evaluate (engine should handle gracefully)

### `OPERATOR_NOT_ALLOWED` (P2, operator-restriction)

- **脚本**: `~1`
  - 触发原因: Bitwise NOT may be disallowed by operator restriction policy

### `PARSE_AST_DEPTH_EXCEEDED` (P0, syntax-parsing)

- **脚本**: `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))`
  - 触发原因: Deeply nested expression beyond parser MAX_PARSE_DEPTH=100 -- returns PARSE_AST_DEPTH_EXCEEDED instead of crashing the worker process (P0 fix: parser recursion depth guard)

### `QL_THROW` (P1, user-exception)

- **脚本**: `throw 'business error'`
  - 触发原因: QLExpress throw statement

### `SCRIPT_TIME_OUT` (P1, resource-limits)

- **脚本**: `while(true) { x = x + 1 }`
  - 触发原因: Infinite loop triggers script timeout

### `SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND` (P2, serialization-cache)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- requires corrupted cache referencing missing class

### `SERIALIZABLE_PARSE_CACHE_INVALID_MODEL` (P2, serialization-cache)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- requires corrupted cache with invalid model

### `SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND` (P2, serialization-cache)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- requires corrupted cache referencing missing operator

### `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT` (P2, serialization-cache)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- requires corrupted cache with unsupported constant

### `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION` (P2, serialization-cache)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- requires corrupted cache with unsupported instruction

### `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION` (P2, serialization-cache)

- **脚本**: *(无法通过纯脚本触发)*
  - 触发原因: Cannot trigger via script -- requires corrupted cache with unsupported version

### `SET_FIELD_UNKNOWN_ERROR` (P1, field-access)

- **脚本**: `x = 1; x.readOnlyField = 99`
  - 触发原因: Attempt to set field on immutable object

### `STACK_OVERFLOW` (P0, resource-limits)

- **脚本**: `function r(n) { return r(n-1); }; r(100000)`
  - 触发原因: Unbounded recursion overflows the call stack

### `WHILE_CONDITION_BOOL_REQUIRED` (P1, control-flow)

- **脚本**: `while (42) { x = 1 }`
  - 触发原因: while condition must be boolean, got int

### `WHILE_CONDITION_ERROR` (P1, control-flow)

- **脚本**: `while (1/0) { x = 1 }`
  - 触发原因: while condition throws arithmetic error

## 4. 这能告诉你什么 / 不能告诉你什么

### 能告诉你

- **哪些错误码在常见业务模式中可能触发**: 合成语料覆盖了所有 65 个错误码的触发场景，
  给业务脚本验收提供了优先级排序依据。
- **哪些错误码需要特殊环境**: 6 个 SERIALIZABLE_PARSE_CACHE_* 错误码、
  INVALID_ARGUMENT、BIZ_EXCEPTION 无法通过纯 QLExpress 脚本触发，
  需要 Java 端注入损坏缓存或注册自定义函数。
- **错误码的功能域分布**: 从控制流、算术、类型转换到序列化缓存，
  每个域都有对应的合成触发脚本。

### 不能告诉你

- **真实业务里哪些错误码频率最高**: 合成语料是人工构造的，
  不代表真实业务脚本的错误分布。需要用户提供业务脚本进行对比。
- **错误码的实际触发是否符合预期**: 合成语料标注为「理论触发」，
  需要通过 Rust 运行时验证确认实际行为。
- **错误消息是否与 Java 基准一致**: 合成语料只验证错误码，
  不验证错误消息格式。

## 5. 如何使用业务脚本运行对比

如果用户提供了业务脚本，可以按以下步骤运行对比：

### 步骤 1: 准备业务脚本 JSONL

```jsonl
{"id": "my-biz-001", "script": "score >= 80 ? 'pass' : 'fail'", "context": {"score": 90}}
{"id": "my-biz-002", "script": "items[0] + items[1]", "context": {"items": [10, 20]}}
```

### 步骤 2: 运行差分对比

```bash
# 使用 Rust 运行时验证合成语料
cargo run --bin run-script-biz -- \
  --corpus verification/corpus/business-synthetic.jsonl \
  --output verification/results/synthetic-results.json

# 使用 Java 基准对比
java -cp QLExpress/target/test-classes \
  com.alibaba.qlexpress4.TestRunner \
  --corpus verification/corpus/business-synthetic.jsonl
```

### 步骤 3: 分析覆盖差异

```bash
# 对比 Rust vs Java 的错误码触发差异
python3 scripts/analyze_error_distribution.py
```

