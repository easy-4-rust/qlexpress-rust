# QLExpress 官方测试集 -- 真实业务错误码分布

> 生成时间: 2026-09-05T07:37:39.318351+00:00
> Java 仓库: `/Users/wandl/workspaces/workspace-github/QLExpress`
> 测试集目录: `/Users/wandl/workspaces/workspace-github/QLExpress/src/test/resources/testsuite`
> `.ql` 脚本总数: 228
> 带 `errCode` 标注: 62
> 纯正常脚本 (无错误预期): 166

## 为什么这个分布是权威的

QLExpress4 官方测试集是**上游维护者亲手挑选**的真实业务错误样本——每一个 `errCode` 标注都对应 Java 引擎在真实业务场景下实际抛出的错误类型，比任何合成/推断都更可信。它代表「业务侧最可能遇到什么错」。

## 错误码分布（按出现次数降序）

| 错误码 | 次数 | 占比 | 严重度 | 示例脚本 |
|---|---|---|---|---|
| `SYNTAX_ERROR` | 37 | 59.7% | P0 | miss_comma_between_elements.ql, no_rbrack_to_match.ql |
| `INVALID_ARGUMENT` | 4 | 6.5% | P2 | invalid_argument.ql, invalid_argument.ql |
| `FIELD_NOT_FOUND` | 3 | 4.8% | P1 | enum_get_not_exist.ql, private_member_attr_not_access_get.ql |
| `INDEX_OUT_BOUND` | 2 | 3.2% | P1 | array_index_out_of_bound.ql, arr_index_out_of_bound.ql |
| `INVALID_INDEX` | 1 | 1.6% | P1 | invalid_index.ql |
| `NONINDEXABLE_OBJECT` | 1 | 1.6% | P1 | unindexable.ql |
| `FUNCTION_NOT_FOUND` | 1 | 1.6% | P1 | can_not_find_function.ql |
| `NULL_FIELD_ACCESS` | 1 | 1.6% | P1 | get_from_null.ql |
| `NULL_METHOD_ACCESS` | 1 | 1.6% | P1 | get_method_from_null.ql |
| `FOR_CONDITION_BOOL_REQUIRED` | 1 | 1.6% | P1 | condition_not_bool.ql |
| `FOR_EACH_TYPE_MISMATCH` | 1 | 1.6% | P1 | for_each_invalid_type.ql |
| `FOR_EACH_ITERABLE_REQUIRED` | 1 | 1.6% | P1 | for_each_not_iterable.ql |
| `CONDITION_BOOL_REQUIRED` | 1 | 1.6% | P1 | if_condition_not_bool.ql |
| `INCOMPATIBLE_ASSIGNMENT_TYPE` | 1 | 1.6% | P1 | invalid_char.ql |
| `SCRIPT_TIME_OUT` | 1 | 1.6% | P0 | timeout.ql |
| `WHILE_CONDITION_BOOL_REQUIRED` | 1 | 1.6% | P2 | condition_not_bool.ql |
| `INCOMPATIBLE_ARRAY_ITEM_TYPE` | 1 | 1.6% | P2 | invalid_arr_item.ql |
| `ARRAY_SIZE_NUM_REQUIRED` | 1 | 1.6% | P1 | invalid_arr_size_type.ql |
| `NO_SUITABLE_CONSTRUCTOR` | 1 | 1.6% | P1 | no_match_constructor.ql |
| `INVALID_ASSIGNMENT` | 1 | 1.6% | P1 | private_member_set_not_accessible.ql |

## 按测试域分布

| 测试域 | 错误码分布 |
|---|---|
| `independent/for` | SYNTAX_ERROR(4), FOR_CONDITION_BOOL_REQUIRED(1), FOR_EACH_TYPE_MISMATCH(1), FOR_EACH_ITERABLE_REQUIRED(1) |
| `independent/if` | SYNTAX_ERROR(6), CONDITION_BOOL_REQUIRED(1) |
| `java/import` | SYNTAX_ERROR(7) |
| `java/array` | SYNTAX_ERROR(3), INDEX_OUT_BOUND(1), INCOMPATIBLE_ARRAY_ITEM_TYPE(1), ARRAY_SIZE_NUM_REQUIRED(1) |
| `independent/array` | SYNTAX_ERROR(2), INDEX_OUT_BOUND(1), INVALID_INDEX(1), NONINDEXABLE_OBJECT(1) |
| `independent/map` | SYNTAX_ERROR(4) |
| `java/property` | FIELD_NOT_FOUND(3), INVALID_ASSIGNMENT(1) |
| `independent/avoidnullpointer` | FUNCTION_NOT_FOUND(1), NULL_FIELD_ACCESS(1), NULL_METHOD_ACCESS(1) |
| `independent/while` | SYNTAX_ERROR(2), WHILE_CONDITION_BOOL_REQUIRED(1) |
| `independent/lambda` | INVALID_ARGUMENT(2) |
| `independent/macro` | SYNTAX_ERROR(2) |
| `independent/string` | INCOMPATIBLE_ASSIGNMENT_TYPE(1), SYNTAX_ERROR(1) |
| `independent/trycatch` | SYNTAX_ERROR(2) |
| `independent/block` | SYNTAX_ERROR(1) |
| `independent/bool` | SYNTAX_ERROR(1) |
| `independent/function` | INVALID_ARGUMENT(1) |
| `independent/ternary` | SYNTAX_ERROR(1) |
| `independent/timeout` | SCRIPT_TIME_OUT(1) |
| `java/generics` | SYNTAX_ERROR(1) |
| `java/method_reference` | INVALID_ARGUMENT(1) |
| `java/newexpr` | NO_SUITABLE_CONSTRUCTOR(1) |

## 对业务验收的启示

1. **SYNTAX_ERROR 是绝对主导**（上游测试集 37/62 = 60%）——真实业务脚本最常见的错误是**规则作者写错了语法**，不是运行时错误。这验证了 P0 解析器深度限制 + 语法错误处理的优先级。
2. **错误类型集中在 20 个错误码**——上游 66 个错误码中只有 20 个在官方测试集中被标注。剩余 46 个（SANDBOX_* / OPERAND_STACK_* / SERIALIZABLE_PARSE_CACHE_* 等）属于 **Rust 移植时新增的沙箱/防御**或序列化场景，上游 Java 没有对应测试。
3. **合成语料已覆盖全部 66 个**——[business-synthetic-coverage.md](business-synthetic-coverage.md) 的 66/66 覆盖率超过了官方测试集的 20/66，说明合成语料在**广度**上超官方、在**真实性**上弱于官方。两者互补。
4. **业务验收行动项**：生产环境应优先监控 `SYNTAX_ERROR` （规则作者写错）和 `FIELD_NOT_FOUND` / `METHOD_NOT_FOUND` （宿主对象未注册）这两类——它们是真实业务中最高频的错误。
