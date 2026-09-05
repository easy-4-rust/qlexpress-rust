# QLExpress-Rust 错误码分布报告

> 生成时间: 2026-09-05T07:46:31.317197+00:00
> 语料来源: `verification/corpus/differential.jsonl` (295 条)
> 错误码来源: `crates/qlexpress/src/exception/ql_error_codes.rs` (67 个)

## 1. 总览

| 指标 | 值 |
|---|---|
| 语料总数 | 295 |
| 预期成功 | 257 |
| 预期异常 | 38 |
| 触发的错误码种类 | 9 / 67 |
| 未触发的错误码种类 | 58 |

## 2. 按严重度分布

| 严重度 | 数量 | 占异常比 |
|---|---|---|
| P0 | 1 | 2.6% |
| P1 | 37 | 97.4% |
| P2 | 0 | 0.0% |
| Success | 257 | 87.1% (占总数) |

## 3. 按用例类型分布

| 用例类型 | 数量 |
|---|---|
| script | 120 |
| number_math | 115 |
| operator_manager | 45 |
| delegate_context | 2 |
| fixed_size_stack | 1 |
| runtime_core | 1 |
| exception_table | 1 |
| batch_add_function_result | 1 |
| ql_functional_varargs | 1 |
| lsp_position | 1 |
| lsp_range | 1 |
| lsp_diagnostic | 1 |
| exist_stack | 1 |
| macro_define | 1 |
| user_define_exception | 1 |
| security_strategies | 1 |
| ql_string_utils | 1 |

## 4. 错误码触发明细

| 错误码 | 触发次数 | 占异常比 | 严重度 |
|---|---|---|---|
| `INVALID_BINARY_OPERAND` | 16 | 42.1% | P1 |
| `INVALID_ARITHMETIC` | 12 | 31.6% | P1 |
| `NULL_FIELD_ACCESS` | 4 | 10.5% | P1 |
| `EXCEED_MAX_ARR_LENGTH` | 1 | 2.6% | P1 |
| `FUNCTION_NOT_FOUND` | 1 | 2.6% | P1 |
| `INDEX_OUT_BOUND` | 1 | 2.6% | P1 |
| `INVALID_ASSIGNMENT` | 1 | 2.6% | P1 |
| `INVALID_INDEX` | 1 | 2.6% | P1 |
| `SYNTAX_ERROR` | 1 | 2.6% | P0 |

## 5. 未触发的错误码 (仓库语料未覆盖)

以下错误码在 295 条差分语料中**未被任何用例触发**：

**Syntax & Parsing**:

- `MISSING_INDEX` (P0)
- `INVALID_NUMBER` (P0)
- `CLASS_NOT_FOUND` (P0)

**Stack**:

- `STACK_OVERFLOW` (P0)
- `OPERAND_STACK_OVERFLOW` (P0)
- `OPERAND_STACK_UNDERFLOW` (P0)

**Index & Access**:

- `NONINDEXABLE_OBJECT` (P1)
- `NONTRAVERSABLE_OBJECT` (P1)
- `NULL_METHOD_ACCESS` (P1)
- `FIELD_NOT_FOUND` (P1)
- `SET_FIELD_UNKNOWN_ERROR` (P1)
- `GET_FIELD_UNKNOWN_ERROR` (P1)

**Function / Method Invocation**:

- `INVOKE_METHOD_WITH_WRONG_ARGUMENTS` (P1)
- `INVOKE_METHOD_INNER_ERROR` (P1)
- `INVOKE_METHOD_UNKNOWN_ERROR` (P1)
- `INVOKE_FUNCTION_INNER_ERROR` (P1)
- `FUNCTION_TYPE_MISMATCH` (P1)
- `INVOKE_LAMBDA_ERROR` (P1)
- `NULL_CALL` (P1)
- `OBJECT_NOT_CALLABLE` (P1)
- `METHOD_NOT_FOUND` (P1)
- `INVOKE_CONSTRUCTOR_UNKNOWN_ERROR` (P1)
- `INVOKE_CONSTRUCTOR_INNER_ERROR` (P1)
- `NO_SUITABLE_CONSTRUCTOR` (P1)

**Block & Control Flow**:

- `EXECUTE_BLOCK_ERROR` (P1)
- `FOR_EACH_ITERABLE_REQUIRED` (P1)
- `FOR_EACH_TYPE_MISMATCH` (P1)
- `FOR_EACH_UNKNOWN_ERROR` (P1)
- `FOR_INIT_ERROR` (P1)
- `FOR_BODY_ERROR` (P1)
- `FOR_UPDATE_ERROR` (P1)
- `FOR_CONDITION_ERROR` (P1)
- `FOR_CONDITION_BOOL_REQUIRED` (P1)
- `WHILE_CONDITION_BOOL_REQUIRED` (P1)
- `WHILE_CONDITION_ERROR` (P1)
- `CONDITION_BOOL_REQUIRED` (P1)

**Type Cast & Assignment**:

- `INCOMPATIBLE_TYPE_CAST` (P1)
- `INVALID_CAST_TARGET` (P1)
- `INCOMPATIBLE_ASSIGNMENT_TYPE` (P1)

**Arithmetic & Operators**:

- `EXECUTE_OPERATOR_EXCEPTION` (P1)
- `INVALID_UNARY_OPERAND` (P1)

**Array**:

- `ARRAY_SIZE_NUM_REQUIRED` (P1)
- `INCOMPATIBLE_ARRAY_ITEM_TYPE` (P1)

**Try / Catch / Finally**:

- `EXECUTE_FINAL_BLOCK_ERROR` (P1)
- `EXECUTE_TRY_BLOCK_ERROR` (P1)
- `EXECUTE_CATCH_HANDLER_ERROR` (P1)

**Timeout**:

- `SCRIPT_TIME_OUT` (P1)

**Operator Restriction**:

- `OPERATOR_NOT_ALLOWED` (P2)

**Serializable Parse Cache**:

- `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION` (P2)
- `SERIALIZABLE_PARSE_CACHE_INVALID_MODEL` (P2)
- `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION` (P2)
- `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT` (P2)
- `SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND` (P2)
- `SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND` (P2)

**User Defined Exception**:

- `INVALID_ARGUMENT` (P1)
- `BIZ_EXCEPTION` (P1)
- `QL_THROW` (P1)

## 6. 按功能域覆盖情况

| 功能域 | 已覆盖 | 未覆盖 | 覆盖率 |
|---|---|---|---|
| Syntax & Parsing | 1 | 3 | 25% |
| Stack | 0 | 3 | 0% |
| Index & Access | 3 | 6 | 33% |
| Function / Method Invocation | 1 | 12 | 8% |
| Block & Control Flow | 0 | 12 | 0% |
| Type Cast & Assignment | 1 | 3 | 25% |
| Arithmetic & Operators | 2 | 2 | 50% |
| Array | 1 | 2 | 33% |
| Try / Catch / Finally | 0 | 3 | 0% |
| Timeout | 0 | 1 | 0% |
| Operator Restriction | 0 | 1 | 0% |
| Serializable Parse Cache | 0 | 6 | 0% |
| User Defined Exception | 0 | 3 | 0% |

## 7. 异常用例明细

| 用例 ID | 用例类型 | 错误码 | 严重度 |
|---|---|---|---|
| `null-value` | script | `NULL_FIELD_ACCESS` | P1 |
| `avoid-null` | script | `NULL_FIELD_ACCESS` | P1 |
| `array-limit-error` | script | `EXCEED_MAX_ARR_LENGTH` | P1 |
| `divide-by-zero-error` | script | `INVALID_ARITHMETIC` | P1 |
| `missing-function-error` | script | `FUNCTION_NOT_FOUND` | P1 |
| `null-field-error` | script | `NULL_FIELD_ACCESS` | P1 |
| `invalid-index-error` | script | `INVALID_INDEX` | P1 |
| `index-out-of-bound-error` | script | `INDEX_OUT_BOUND` | P1 |
| `syntax-error` | script | `SYNTAX_ERROR` | P0 |
| `number-int-overflow-add` | script | `INVALID_ARITHMETIC` | P1 |
| `number-int-overflow-multiply` | script | `INVALID_ARITHMETIC` | P1 |
| `number-long-overflow-add` | script | `INVALID_ARITHMETIC` | P1 |
| `number-long-overflow-subtract` | script | `INVALID_ARITHMETIC` | P1 |
| `number-shift-float-left-error` | script | `INVALID_BINARY_OPERAND` | P1 |
| `number-shift-float-distance-error` | script | `INVALID_BINARY_OPERAND` | P1 |
| `numbermath-add-int-overflow` | number_math | `INVALID_ARITHMETIC` | P1 |
| `numbermath-error-double-bitwise` | number_math | `INVALID_BINARY_OPERAND` | P1 |
| `numbermath-error-float-shift-distance` | number_math | `INVALID_BINARY_OPERAND` | P1 |
| `numbermath-error-bigint-unsigned-shift` | number_math | `INVALID_BINARY_OPERAND` | P1 |
| `numbermath-error-integer-divide-zero` | number_math | `INVALID_ARITHMETIC` | P1 |
| `numbermath-error-nonpositive-modulus` | number_math | `INVALID_ARITHMETIC` | P1 |
| `basebinary-multiply-overflow` | script | `INVALID_ARITHMETIC` | P1 |
| `basebinary-multiply-char-error` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-divide-zero-error-reason` | script | `INVALID_ARITHMETIC` | P1 |
| `basebinary-divide-floating-zero` | script | `INVALID_ARITHMETIC` | P1 |
| `basebinary-remainder-zero-error-reason` | script | `INVALID_ARITHMETIC` | P1 |
| `basebinary-bitwise-and-boolean-null` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-bitwise-or-null-boolean` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-bitwise-xor-null-boolean` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-in-null-null` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-in-null-list` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-in-invalid-map-error` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-like-null-null` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-like-one-null` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-like-invalid-error` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-invalid-operand-reason` | script | `INVALID_BINARY_OPERAND` | P1 |
| `basebinary-invalid-left-value` | script | `INVALID_ASSIGNMENT` | P1 |
| `operator-manager-precedence-missing-java-implementation-divergence` | operator_manager | `NULL_FIELD_ACCESS` | P1 |

## 8. 对业务验收的启示

### 高频错误码需单独验证

以下错误码在仓库语料中被高频触发，业务脚本应额外覆盖其边界条件：

- **`INVALID_BINARY_OPERAND`** (触发 16 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`INVALID_ARITHMETIC`** (触发 12 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`NULL_FIELD_ACCESS`** (触发 4 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`EXCEED_MAX_ARR_LENGTH`** (触发 1 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`FUNCTION_NOT_FOUND`** (触发 1 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`INDEX_OUT_BOUND`** (触发 1 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`INVALID_ASSIGNMENT`** (触发 1 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`INVALID_INDEX`** (触发 1 次) -- 需在业务脚本中验证不同输入模式下的触发路径
- **`SYNTAX_ERROR`** (触发 1 次) -- 需在业务脚本中验证不同输入模式下的触发路径

### 未覆盖错误码需补充验证

**P0 (编译/沙箱/栈) 未覆盖** -- 这些是编译期或基础设施错误，
业务脚本需要专门构造触发条件：

- `CLASS_NOT_FOUND`
- `INVALID_NUMBER`
- `MISSING_INDEX`
- `OPERAND_STACK_OVERFLOW`
- `OPERAND_STACK_UNDERFLOW`
- `PARSE_AST_DEPTH_EXCEEDED`
- `STACK_OVERFLOW`

**P1 (运行时) 未覆盖** -- 这些是运行时错误，
业务脚本应通过构造特定输入来覆盖：

- `ARRAY_SIZE_NUM_REQUIRED`
- `BIZ_EXCEPTION`
- `CONDITION_BOOL_REQUIRED`
- `EXECUTE_BLOCK_ERROR`
- `EXECUTE_CATCH_HANDLER_ERROR`
- `EXECUTE_FINAL_BLOCK_ERROR`
- `EXECUTE_OPERATOR_EXCEPTION`
- `EXECUTE_TRY_BLOCK_ERROR`
- `FIELD_NOT_FOUND`
- `FOR_BODY_ERROR`
- `FOR_CONDITION_BOOL_REQUIRED`
- `FOR_CONDITION_ERROR`
- `FOR_EACH_ITERABLE_REQUIRED`
- `FOR_EACH_TYPE_MISMATCH`
- `FOR_EACH_UNKNOWN_ERROR`
- `FOR_INIT_ERROR`
- `FOR_UPDATE_ERROR`
- `FUNCTION_TYPE_MISMATCH`
- `GET_FIELD_UNKNOWN_ERROR`
- `INCOMPATIBLE_ARRAY_ITEM_TYPE`
- `INCOMPATIBLE_ASSIGNMENT_TYPE`
- `INCOMPATIBLE_TYPE_CAST`
- `INVALID_ARGUMENT`
- `INVALID_CAST_TARGET`
- `INVALID_UNARY_OPERAND`
- `INVOKE_CONSTRUCTOR_INNER_ERROR`
- `INVOKE_CONSTRUCTOR_UNKNOWN_ERROR`
- `INVOKE_FUNCTION_INNER_ERROR`
- `INVOKE_LAMBDA_ERROR`
- `INVOKE_METHOD_INNER_ERROR`
- `INVOKE_METHOD_UNKNOWN_ERROR`
- `INVOKE_METHOD_WITH_WRONG_ARGUMENTS`
- `METHOD_NOT_FOUND`
- `NONINDEXABLE_OBJECT`
- `NONTRAVERSABLE_OBJECT`
- `NO_SUITABLE_CONSTRUCTOR`
- `NULL_CALL`
- `NULL_METHOD_ACCESS`
- `OBJECT_NOT_CALLABLE`
- `QL_THROW`
- `SCRIPT_TIME_OUT`
- `SET_FIELD_UNKNOWN_ERROR`
- `WHILE_CONDITION_BOOL_REQUIRED`
- `WHILE_CONDITION_ERROR`

**P2 (基础设施/缓存) 未覆盖** -- 这些错误涉及序列化缓存和操作符限制，
可在集成测试阶段覆盖：

- `OPERATOR_NOT_ALLOWED`
- `SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND`
- `SERIALIZABLE_PARSE_CACHE_INVALID_MODEL`
- `SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND`
- `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT`
- `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION`
- `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION`

### 验收建议

1. **优先补充 P0 未覆盖错误码**：编译期错误对用户体验影响最大
2. **高频 P1 错误码需多角度验证**：同一错误码的不同触发路径可能暴露不同的 bug
3. **P2 错误码可在集成测试中覆盖**：序列化缓存等场景适合端到端测试
4. **成功率基线**：当前语料预期成功率 87.1%，异常率 12.9%

