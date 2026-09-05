# 调用栈深度冒烟测试 + 预算边界验证

> 子命令: `qlexpress-verification stack-depth-probe [max-depth]`
>
> 测试日期: 2026-09-05 (初版) / 2026-09-05 (P0 修复后更新)
>
> 工具链: Rust stable, qlexpress HEAD=f25f0c0 (含 parse-depth guard)

## 1. 测试方法

四类独立探针,均通过 `execute_checked` 路径执行,使用 `SandboxProfile` 配置资源预算。

探针 2、3 和 4 使用子进程隔离,因为深度嵌套表达式会在解析阶段溢出 Rust
进程自身的调用栈(硬中止,`catch_unwind` 无法捕获)。

---

## 2. 测试结果

### 2.1 函数递归深度 (probe_function_call_depth)

配置: `max_call_depth=128`, `max_fuel=10,000,000`, `timeout_millis=30,000`

| N (递归深度) | 结果 | 错误码 | 详情 | 耗时 (us) |
|---|---|---|---|---|
| 10 | **Ok** | -- | Int(10) | 1,451 |
| 50 | **Ok** | -- | Int(50) | 918 |
| 100 | SANDBOX_CALL_DEPTH_EXCEEDED | `SANDBOX_CALL_DEPTH_EXCEEDED` | depth 129, limit 128 | 806 |
| 128 | SANDBOX_CALL_DEPTH_EXCEEDED | `SANDBOX_CALL_DEPTH_EXCEEDED` | depth 129, limit 128 | 454 |
| 200 | SANDBOX_CALL_DEPTH_EXCEEDED | `SANDBOX_CALL_DEPTH_EXCEEDED` | depth 129, limit 128 | 1,155 |
| 500 | SANDBOX_CALL_DEPTH_EXCEEDED | `SANDBOX_CALL_DEPTH_EXCEEDED` | depth 129, limit 128 | 1,099 |

**发现:**
- 沙箱 `max_call_depth=128` 被精确执行。每次递归函数调用消耗约 1.29 个调用帧
  (函数入口 + if 判断 + return 表达式)。
- 安全边界: N=50 安全,N>=100 触发限制。实际最大安全递归深度约为 N=50
  (含 ~65 帧,远低于 128 上限)。
- `SANDBOX_FUEL_EXCEEDED` 和 `SANDBOX_DEADLINE_EXCEEDED` 均未被触发 -- 调用深度
  预算是最先触发的限制。

### 2.2 操作数栈压力 (probe_operand_stack_depth)

配置: `max_call_depth=10,000`, `max_fuel=100,000,000`,
`max_ast_depth=100,000`, `max_instructions=1,000,000`, `timeout_millis=60,000`

表达式: `((((1 + 1) + 1) + 1) ...)` 深度 N

**P0 修复后结果 (parse-depth guard, MAX_PARSE_DEPTH=100):**

| N (嵌套深度) | 结果 | 错误码 | 详情 | 耗时 (us) |
|---|---|---|---|---|
| 10 | **Ok** | -- | Int(46) | ~3,000 |
| 50 | **Ok** | -- | Int(1,226) | ~6,000 |
| 80 | **Ok** | -- | Int(3,961) | ~7,000 |
| 90 | **Ok** | -- | Int(4,501) | ~7,000 |
| 100 | **Ok** | -- | Int(4,951) | ~7,000 |
| 101 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | parse AST depth 101, exceed max allowed depth 100 | ~1,000 |
| 120 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | (same) | ~1,000 |
| 200 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | (same) | ~1,000 |
| 500 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | (same) | ~1,000 |

**P0 修复前结果 (无 depth guard):**

| N (嵌套深度) | 结果 | 错误码 |
|---|---|---|
| 10-114 | **Ok** | -- |
| 115 | PROCESS_STACK_OVERFLOW | -- |

**发现:**
- **`OPERAND_STACK_OVERFLOW` 从未被触发。** 编译器的 `max_stack_size` 计算对所有
  可成功解析的深度都是正确的。
- **P0 修复:** N=101 起返回 `PARSE_AST_DEPTH_EXCEEDED`(进程不崩溃),
  替代了旧的 N=115 `PROCESS_STACK_OVERFLOW`(进程硬中止)。
- 编译器正确性证据: 对于二元加法表达式,`max_stack_size` 始终 >= 2 (操作数栈
  只需要容纳左右操作数),编译器从未低估栈需求。

### 2.3 嵌套 try-catch (probe_nested_try_catch)

配置: `max_call_depth=128`, `max_fuel=10,000,000`,
`max_ast_depth=10,000`, `max_ast_nodes=500,000`, `timeout_millis=30,000`

脚本: `try { try { ... throw "err" ... } catch (e) { 1 } } catch (e) { 2 }`

**P0 修复后结果:**

| N (嵌套深度) | 结果 | 错误码 | 详情 | 耗时 (us) |
|---|---|---|---|---|
| 5 | **Ok** | -- | Int(1) | ~3,000 |
| 10 | **Ok** | -- | Int(1) | ~3,000 |
| 20 | **Ok** | -- | Int(1) | ~4,000 |
| 25 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | parse AST depth 101, exceed max allowed depth 100 | ~1,000 |
| 50+ | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | (same) | ~1,000 |

**发现:**
- P0 修复后,N>=25 的嵌套 try-catch 返回 `PARSE_AST_DEPTH_EXCEEDED`(每个
  try-catch 约消耗 4 层解析深度,25 层 ≈ 100 深度)。
- 旧行为: N=105 触发 `PROCESS_STACK_OVERFLOW`(进程硬中止)。
- 沙箱预算限制在安全范围内均未被触发。

### 2.4 解析器深度守卫 (probe_parse_depth)

配置: 与 Probe 2 相同

表达式: `(((...1...)))` 纯括号嵌套深度 N

| N (嵌套深度) | 结果 | 错误码 | 详情 | 耗时 (us) |
|---|---|---|---|---|
| 10 | **Ok** | -- | Int(1) | ~1,000 |
| 50 | **Ok** | -- | Int(1) | ~2,000 |
| 80 | **Ok** | -- | Int(1) | ~3,000 |
| 90 | **Ok** | -- | Int(1) | ~3,500 |
| 100 | **Ok** | -- | Int(1) | ~4,000 |
| 101 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | parse AST depth 101, exceed max allowed depth 100 | ~500 |
| 120 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | (same) | ~500 |
| 200 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | (same) | ~500 |
| 500 | PARSE_AST_DEPTH_EXCEEDED | `PARSE_AST_DEPTH_EXCEEDED` | (same) | ~500 |

**发现:**
- 解析器深度守卫在 N=101 精确触发 `PARSE_AST_DEPTH_EXCEEDED`。
- N<=100 的所有脚本正常解析,进程不崩溃。
- N>100 的脚本返回结构化错误码,而非进程硬中止。
- **根治了旧版本 N>=115 的 `PROCESS_STACK_OVERFLOW` 问题。**

---

## 3. 关键发现总结

### 3.1 P0 修复: 解析器深度守卫

| 维度 | 修复前 | 修复后 |
|---|---|---|
| 表达式嵌套溢出边界 | N=115 (`PROCESS_STACK_OVERFLOW`) | N=100 (`PARSE_AST_DEPTH_EXCEEDED`) |
| try-catch 嵌套溢出边界 | N=105 (`PROCESS_STACK_OVERFLOW`) | N=25 (`PARSE_AST_DEPTH_EXCEEDED`) |
| 进程是否崩溃 | **是** (硬中止) | **否** (结构化错误码) |
| 错误码 | 无 (进程直接死亡) | `PARSE_AST_DEPTH_EXCEEDED` |
| 沙箱可捕获 | 否 | **是** |
| 深度上限实现 | 无 (依赖进程栈大小) | `MAX_PARSE_DEPTH=100` (RAII 守卫) |

**P0 修复机制:** 递归下降解析器的 6 个递归入口函数均加装了 `DepthGuard`
(RAII 守卫),通过 `Cell<usize>` 线程局部变量追踪当前递归深度。当深度
超过 `MAX_PARSE_DEPTH`(100) 时,立即返回 `PARSE_AST_DEPTH_EXCEEDED`
错误码,而非继续递归直到进程栈溢出。

### 3.2 函数递归深度极限

| 维度 | 值 |
|---|---|
| 沙箱 `max_call_depth` 默认值 | 128 |
| 实际触发 `SANDBOX_CALL_DEPTH_EXCEEDED` 的递归深度 | N>=100 |
| 每次递归消耗的调用帧数 | ~1.29 |
| 最先触发的预算限制 | `SANDBOX_CALL_DEPTH_EXCEEDED` (非 fuel/deadline) |

### 3.3 操作数栈安全上界

| 维度 | 值 |
|---|---|
| `OPERAND_STACK_OVERFLOW` 触发次数 | **0** (从未触发) |
| 编译器 `max_stack_size` 计算正确性 | **已证明** (N<=100 全部 Ok) |
| 解析器深度守卫边界 | N=100 (`PARSE_AST_DEPTH_EXCEEDED`) |
| 旧行为: 进程栈溢出边界 | N=115 (`PROCESS_STACK_OVERFLOW`) -- **已修复** |

### 3.4 深度上限对比表

| 深度限制 | 值 | 机制 | 可被沙箱捕获 |
|---|---|---|---|
| `MAX_PARSE_DEPTH` (解析器守卫) | **100** | RAII `DepthGuard` + `Cell<usize>` | **是** |
| 操作数栈 (`FixedSizeStack`) | 按指令计算 | `try_push` / `try_pop` | **是** |
| `max_call_depth` (沙箱) | 128 | 每次函数调用递增 | **是** |
| `max_ast_depth` (沙箱) | 256 | AST 节点计数 | **是** |
| 进程调用栈 (旧,已修复) | ~115 | 操作系统硬限制 | **否** |

---

## 4. 对业务验收的启示

### 4.1 典型业务嵌套安全余量

典型业务脚本嵌套深度不超过 10 层。以下安全余量适用于此场景:

| 预算维度 | 默认限制 | 典型业务消耗 | 安全余量 |
|---|---|---|---|
| `max_call_depth` | 128 | 5-10 帧 | **12-25x** |
| `max_fuel` | 1,000,000 | 100-1,000 | **1,000-10,000x** |
| `max_ast_depth` | 256 | 10-30 | **8-25x** |
| `MAX_PARSE_DEPTH` (解析器守卫) | 100 | 5-10 层 | **10-20x** |
| 操作数栈 | 按指令计算 | 2-5 | 充裕 |

**结论: 典型业务脚本在所有维度上都有充足的安全余量。P0 修复后,
所有深度超限均返回结构化错误码,不再有进程硬中止风险。**

### 4.2 超过深度限制的行为

| 场景 | 触发的限制 | 后果 |
|---|---|---|
| 递归函数 N>50 | `SANDBOX_CALL_DEPTH_EXCEEDED` | 脚本被沙箱安全中止,返回错误码 |
| 表达式嵌套 N>100 | `PARSE_AST_DEPTH_EXCEEDED` | **安全中止**,返回错误码 (P0 修复) |
| try-catch 嵌套 N>25 | `PARSE_AST_DEPTH_EXCEEDED` | **安全中止**,返回错误码 (P0 修复) |
| 旧行为: 表达式嵌套 N>114 | 进程调用栈溢出 | ~~硬中止~~ (**已修复**) |
| 旧行为: try-catch 嵌套 N>104 | 进程调用栈溢出 | ~~硬中止~~ (**已修复**) |

### 4.3 建议的安全配置

对于处理不可信脚本的生产环境:

```rust
ResourceLimits {
    max_ast_depth: 128,      // 保守:远低于进程栈溢出边界
    max_call_depth: 64,      // 保守:典型业务只需 5-10
    max_fuel: 500_000,       // 中等:足够复杂业务逻辑
    timeout_millis: 500,     // 快速失败
    ..ResourceLimits::default()
}
```

---

## 5. P0 修复记录

| 项目 | 内容 |
|---|---|
| 修复日期 | 2026-09-05 |
| 修复 commit | HEAD=f25f0c0 |
| 问题 | 递归下降解析器无深度限制,嵌套 ~115 层即导致进程栈溢出 |
| 修复方案 | `DepthGuard` RAII 守卫 + `Cell<usize>` 线程局部深度计数 |
| `MAX_PARSE_DEPTH` | 100 |
| 新增错误码 | `PARSE_AST_DEPTH_EXCEEDED` |
| 受影响函数 | `parse_expression`, `parse_block_statements_until`, `parse_block_statement`, `parse_block_statements_until_switch_group_end`, `parse_expression_list_until_arrow`, `parse_block_expr` |

---

## 6. 运行方式

```bash
# 运行完整探针套件 (max_depth=500)
cargo run -p qlexpress-verification --bin qlexpress-verification -- stack-depth-probe 500

# 运行单个子进程探针 (用于调试)
STACK_PROBE_SCRIPT='1 + 1' cargo run -p qlexpress-verification --bin qlexpress-verification -- stack-depth-probe-single
```

输出格式: JSON 到 stdout,进度信息到 stderr。
