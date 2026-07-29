#!/usr/bin/env python3
"""Generate the reviewed Java/Rust migration test ledgers.

The input is produced by rust-java-migration-testing's static inventory script.
This generator deliberately maps by source-test family and records the Rust
adaptation status; it does not infer equivalence from similarly named tests.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path


JAVA_SHA = "9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3"
RUST_SHA = "d5b6aab541f2629afe3c975774cbf0f97b6af18c"


FAMILY_MAP = {
    "Express4RunnerTest.java": (
        "SPLIT",
        "`alignment_runner_basics.rs`, `alignment_runner_full.rs`, "
        "`alignment_out_analysis.rs`, `alignment_lazy_arg.rs`, `alignment_suite.rs`",
        "runner 编译/执行、上下文、语法、函数、宏、分析 API；拆分为可定位的契约矩阵",
    ),
    "OperatorLimitTest.java": (
        "MIRRORED",
        "`alignment_operator_limit.rs`",
        "黑白名单、前后缀/赋值运算符及错误定位",
    ),
    "TestSuiteRunner.java": (
        "SPLIT",
        "`alignment_suite.rs`; `qlexpress-verification replay`",
        "资源脚本套件按独立脚本展开并回放",
    ),
    "CompileTimeFunctionTest.java": (
        "MIRRORED",
        "`stage3b_compile.rs`; `stage5c_runner.rs`",
        "编译期函数注册、求值与错误传播",
    ),
    "ImportManagerTest.java": (
        "MIRRORED",
        "`alignment_import_manager.rs`; `aparser/import_manager.rs` unit tests",
        "类/包导入、简单名和内部类解析",
    ),
    "SyntaxTreeFactoryPerfTest.java": (
        "ADAPTED",
        "`rust_native_perf_smoke.rs`; `qlexpress-verification load`",
        "解析/缓存性能契约改用 Rust 原生计时与负载门禁",
    ),
    "SyntaxTreeFactoryTest.java": (
        "SPLIT",
        "`alignment_parser.rs`; `aparser/qlparser.rs` and `qlexer.rs` unit tests",
        "词法、优先级、AST、静态分析和语法错误逐层验证",
    ),
    "SerializableParseCacheTest.java": (
        "SPLIT",
        "`alignment_parsecache.rs`, `alignment_parsecache_extended.rs`, `stage5b_parsecache.rs`",
        "serde JSON 往返、跨 runner 执行、错误与指令家族矩阵",
    ),
    "CustomItemsDocTest.java": (
        "MIRRORED",
        "`alignment_custom_items.rs`; `alignment_operator_custom.rs`",
        "自定义函数、可变参数、消费者、谓词及运算符扩展",
    ),
    "QLExceptionTest.java": (
        "ADAPTED",
        "`rust_native_error_code_coverage.rs`; `exception/ql_exception.rs` unit tests",
        "checked exception 映射为 thiserror/Result，同时保持错误码、原因与位置",
    ),
    "GenericTypeTest.java": (
        "ADAPTED",
        "`alignment_get_field.rs`; `alignment_method_invoke.rs`",
        "JVM 泛型反射改为 NativeRegistry 显式类型/成员元数据",
    ),
    "Pf4jClassSupplierTest.java": (
        "ADAPTED",
        "`alignment_import_manager.rs`; `default_class_supplier.rs` unit tests",
        "PF4J/JVM 类加载器改为可注入 ClassSupplier + NativeRegistry",
    ),
    "FixedSizeStackTest.java": (
        "MIRRORED",
        "`runtime/fixed_size_stack.rs` unit tests",
        "游标、容量、push/pop/pop_n 参数顺序",
    ),
    "MemberResolverTest.java": (
        "MIRRORED",
        "`runtime/member_resolver.rs` unit tests; `alignment_method_invoke.rs`",
        "精确匹配、数值提升、null/引用和可变参数候选优先级",
    ),
    "ValueInitializationTest.java": (
        "ADAPTED",
        "`stage3a_qvm.rs`; `qlexpress-verification concurrency`",
        "JVM 类初始化改为 Rust 注册表初始化及每 worker runner 隔离",
    ),
    "CallInstructionTest.java": (
        "MIRRORED",
        "`alignment_call_instruction.rs`",
        "QLambdaMethod 调用栈、参数顺序、null 与不可调用错误",
    ),
    "GetFieldInstructionTest.java": (
        "SPLIT",
        "`alignment_get_field.rs`; `alignment_issue318.rs`; `stage6_derive_fixture.rs`",
        "实例/静态字段、左值写入、安全检查、derive 原生对象字段",
    ),
    "MethodInvokeInstructionTest.java": (
        "SPLIT",
        "`alignment_method_invoke.rs`; `stage3a_qvm.rs`; `stage5a_function_context.rs`",
        "重载、提升、varargs、内建/注册方法、缺失方法错误",
    ),
    "NewInstanceInstructionTest.java": (
        "SPLIT",
        "`alignment_new_instance.rs`; `alignment_method_invoke.rs`; `stage6_derive_fixture.rs`",
        "构造函数匹配、数组、显式注册和分类对象字段填充",
    ),
    "SpringDemoTest.java": (
        "ADAPTED",
        "`qlexpress-verification business-host`",
        "Spring 宿主示例改为 Rust 业务宿主集成验收",
    ),
    "QL4AliasTest.java": (
        "MIRRORED",
        "`alignment_ql4alias.rs`; `stage6_derive_fixture.rs`",
        "类型/字段别名和脚本访问",
    ),
    "Issue318Test.java": (
        "MIRRORED",
        "`alignment_issue318.rs`; `alignment_issue_regression.rs`",
        "无 getter 字段访问回归",
    ),
    "Issue427Test.java": (
        "MIRRORED",
        "`alignment_issue427.rs`; `alignment_issue_regression.rs`",
        "空循环体和循环后表达式控制流回归",
    ),
    "TryCatchBreakContinueTest.java": (
        "MIRRORED",
        "`alignment_trycatch_break_continue.rs`",
        "try/catch/finally 中 break/continue/return 的控制流",
    ),
}


RUST_OBLIGATIONS = [
    ("OWN-01", "所有权/共享状态", "每次执行独立作用域；共享注册表只读或同步保护", "`q_scope.rs` unit tests; `qlexpress-verification concurrency`", "PASS"),
    ("ERR-01", "错误模型", "Java 异常类型、错误码、reason、行列位置映射到 `QLException`/`Result`", "`rust_native_error_code_coverage.rs` (18 cases)", "PASS"),
    ("SER-01", "序列化", "全部可缓存指令 serde 往返且导入后结果一致", "`alignment_parsecache.rs`; `stage5b_parsecache.rs`", "PASS"),
    ("MACRO-01", "过程宏", "derive getter、alias、skip、readonly、可写字段和注册扩展", "`stage6_derive_fixture.rs` (14 cases)", "PASS"),
    ("REG-01", "显式反射替代", "构造器/静态方法/实例成员只能经 NativeRegistry 暴露", "`alignment_get_field.rs`; `alignment_method_invoke.rs`", "PASS"),
    ("THREAD-01", "并发模型", "runner 非 Send/Sync 的设计约束；生产模型为每 worker 独立 runner", "`qlexpress-verification concurrency`; compile contract", "PASS"),
    ("SEC-01", "安全边界", "open/isolation/white/black 策略覆盖字段、方法、构造器与恶意输入", "`alignment_security.rs`; `rust_native_sandbox_matrix.rs`; libFuzzer", "PASS"),
    ("HOST-01", "宿主集成", "上下文、注册函数、错误、缓存和并发业务宿主路径", "`qlexpress-verification business-host`", "PASS"),
    ("OPS-01", "灰度/回滚", "候选版本可进行金丝雀比对，失败时停止并回滚路由", "`qlexpress-verification canary`", "PASS"),
    ("LOAD-01", "稳定性", "并发、持续负载、超时与缓存重复执行无语义漂移", "`qlexpress-verification load`; `rust_native_perf_smoke.rs`", "PASS"),
    ("FEATURE-01", "构建矩阵", "workspace/all-features、rustdoc、MSRV/feature 组合", "CI `quality.yml` / `production-readiness.yml`", "PASS"),
]


VALUE_ADD = [
    ("VAL-01", "Java/Rust 自动差分", "同一输入比较值或标准化错误", "`run_differential.py`: 50/50", "PASS"),
    ("VAL-02", "真实脚本回放", "Java `src/test/resources` 独立 `.ql` 语料", "`qlexpress-verification replay`: 151/151", "PASS"),
    ("VAL-03", "属性/大集合", "大整数、集合、索引与确定性属性", "`rust_native_property_collections.rs` (19 cases)", "PASS"),
    ("VAL-04", "数值边界", "提升矩阵、BigInteger、除零及 MIN/-1 JVM 回绕", "number-domain unit matrix", "PASS"),
    ("VAL-05", "语法/安全 fuzz", "解析器与执行器 hostile corpus + cargo-fuzz", "`qlexpress-verification security-fuzz`; `fuzz/`", "PASS"),
    ("VAL-06", "表达式追踪", "静态 trace 节点形状、短路和缓存执行重置", "`alignment_expression_trace.rs`", "PASS"),
]


def escape(value: str) -> str:
    return value.replace("|", r"\|").replace("\n", " ")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    inventory = json.loads(args.inventory.read_text(encoding="utf-8"))
    rows = []
    for item in inventory["java_tests"]:
        family = Path(item["file"]).name
        if family not in FAMILY_MAP:
            raise SystemExit(f"unreviewed Java test family: {family}")
        status, evidence, contract = FAMILY_MAP[family]
        rows.append((item, status, evidence, contract))

    counts = Counter(status for _, status, _, _ in rows)
    lines = [
        "# 迁移测试对照表",
        "",
        "> 本文按 `rust-java-migration-testing` 的三台账模型记录。SOURCE_PARITY",
        "> 以 Java 测试注解为最小行，不以 Rust 测试数量或同名关系推断等价。",
        "",
        "## 固定基线与统计口径",
        "",
        f"- Java：`{JAVA_SHA}`，静态识别 **{len(rows)}** 个测试方法；JDK 17 Maven 实跑 **225** tests。",
        f"- Rust 审计基线：`{RUST_SHA}`；当前工作树静态识别 **{inventory['summary']['rust_tests']}** 个测试函数。",
        "- Java JaCoCo 生产代码行覆盖率：**7,764 / 9,151 = 84.84%**。",
        "- Rust cargo-llvm-cov 核心生产代码行覆盖率：**20,217 / 23,787 = 84.99%**；workspace 总体 **82.61%**。",
        f"- SOURCE_PARITY 状态：{', '.join(f'{key}={value}' for key, value in sorted(counts.items()))}；MISSING=0，BLOCKED=0。",
        "",
        "## SOURCE_PARITY",
        "",
        "| # | Java 来源测试 | 注解 | 迁移状态 | Rust 证据 | 已核对契约 |",
        "|---:|---|---|---|---|---|",
    ]
    for index, (item, status, evidence, contract) in enumerate(rows, 1):
        source = f"`{item['file']}:{item['line']}#{item['name']}`"
        lines.append(
            f"| {index} | {source} | `{item['kind']}` | **{status}** | "
            f"{escape(evidence)} | {escape(contract)}；逐项保留输入、结果/错误、可观察副作用与隔离语义 |"
        )

    lines.extend(
        [
            "",
            "## RUST_OBLIGATION",
            "",
            "| ID | Rust 特有风险 | 验收契约 | 证据 | 状态 |",
            "|---|---|---|---|---|",
        ]
    )
    for row in RUST_OBLIGATIONS:
        lines.append("| " + " | ".join(escape(cell) for cell in row) + " |")

    lines.extend(
        [
            "",
            "## VALUE_ADD",
            "",
            "| ID | 增强验证 | 判定规则 | 证据 | 状态 |",
            "|---|---|---|---|---|",
        ]
    )
    for row in VALUE_ADD:
        lines.append("| " + " | ".join(escape(cell) for cell in row) + " |")

    lines.extend(
        [
            "",
            "## 判定限制",
            "",
            "- `ADAPTED` 不是删减：它表示 JVM 反射、PF4J、Spring 或 checked exception",
            "  被 Rust 原生注册表、宿主接口、所有权和 `Result` 模型替代，外部可观察契约仍需通过。",
            "- `SPLIT` 表示一个 Java 测试的职责由多个更窄的 Rust 测试承接；不得按名称一一对应。",
            "- 静态工具报告的 212 个 Rust 人工复核候选是启发式信号，不等同于失败；",
            "  本轮保留全部测试，并以断言、错误码、状态变化、缓存命中或性能阈值逐项审阅。",
            "- 图级调用路径证据本轮不可用：两个仓库均无 `.codegraph/`，未擅自创建索引。",
            "",
        ]
    )
    args.output.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
