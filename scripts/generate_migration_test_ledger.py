#!/usr/bin/env python3
"""Generate method-level Java/Rust migration test ledgers.

The input is produced by rust-java-migration-testing's static inventory script.
Every SOURCE_PARITY row must cite an explicit ``JavaClass#method`` marker in
Rust source. A family-level mapping, a similarly named test, or a green suite is
not accepted as proof for an individual Java method.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from collections import Counter
from pathlib import Path


RUST_OBLIGATIONS = [
    ("OWN-01", "所有权/共享状态", "每次执行独立作用域；共享注册表只读或同步保护", "`q_scope.rs` unit tests; `qlexpress-verification concurrency`", "PENDING"),
    ("ERR-01", "错误模型", "Java 异常类型、错误码、reason、cause、行列位置映射到 `QLException`/`Result`", "`rust_native_error_code_coverage.rs`; Runner cause tests", "PENDING"),
    ("SER-01", "序列化", "全部可缓存指令 serde 往返且导入后结果一致", "`alignment_parsecache.rs`; `stage5b_parsecache.rs`", "PENDING"),
    ("MACRO-01", "过程宏", "derive getter、alias、skip、readonly、可写字段和注册扩展", "`stage6_derive_fixture.rs`", "PENDING"),
    ("REG-01", "显式反射替代", "构造器/静态方法/实例成员只能经 NativeRegistry 暴露", "`alignment_get_field.rs`; `alignment_member_overloads.rs`", "PENDING"),
    ("THREAD-01", "并发模型", "线程本地 Runner + ConcurrentParseCache 首次编译去重", "`java_concurrent_cache_test`; `qlexpress-verification concurrency`", "PENDING"),
    ("SEC-01", "安全边界", "open/isolation/white/black 策略覆盖字段、方法、构造器与恶意输入", "`alignment_security.rs`; fuzz", "PENDING"),
    ("HOST-01", "宿主集成", "上下文、注册函数、错误、缓存和并发业务宿主路径", "`qlexpress-verification business-host`", "PENDING"),
    ("OPS-01", "灰度/回滚", "候选版本可进行金丝雀比对，失败时停止并回滚路由", "`qlexpress-verification canary`", "PENDING"),
    ("LOAD-01", "稳定性", "并发、持续负载、超时与缓存重复执行无语义漂移", "`qlexpress-verification load`", "PENDING"),
    ("FEATURE-01", "构建矩阵", "workspace/all-features、rustdoc、MSRV/feature 组合", "CI workflows", "PENDING"),
]


VALUE_ADD = [
    ("VAL-01", "Java/Rust 自动差分", "同一输入比较值或标准化错误", "`run_differential.py`", "PENDING"),
    ("VAL-02", "真实脚本回放", "Java `src/test/resources` 独立 `.ql` 语料", "`qlexpress-verification replay`", "PENDING"),
    ("VAL-03", "属性/大集合", "大整数、集合、索引与确定性属性", "`rust_native_property_collections.rs`", "PENDING"),
    ("VAL-04", "数值边界", "提升矩阵、BigInteger、除零及 MIN/-1 JVM 回绕", "number-domain unit matrix", "PENDING"),
    ("VAL-05", "语法/安全 fuzz", "解析器与执行器 hostile corpus + cargo-fuzz", "`qlexpress-verification security-fuzz`; `fuzz/`", "PENDING"),
    ("VAL-06", "表达式追踪", "静态 trace 节点形状、短路和缓存执行重置", "`alignment_expression_trace.rs`", "PENDING"),
]


def escape(value: str) -> str:
    return value.replace("|", r"\|").replace("\n", " ")


def braced_body(text: str, start: int) -> str:
    brace = text.find("{", start)
    if brace < 0:
        return ""
    depth = 0
    in_string = False
    escaped = False
    for index in range(brace, len(text)):
        char = text[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[brace : index + 1]
    return text[brace:]


def unique_matches(pattern: str, body: str, limit: int = 8) -> list[str]:
    return list(dict.fromkeys(re.findall(pattern, body)))[:limit]


def source_signals(java_root: Path, item: dict) -> str:
    """Extract review signals from the exact Java method body, not its name alone."""
    path = java_root / item["file"]
    text = path.read_text(encoding="utf-8", errors="replace")
    line_offset = sum(len(line) for line in text.splitlines(keepends=True)[: item["line"] - 1])
    method = re.search(rf"\b{re.escape(item['name'])}\s*\(", text[line_offset:])
    body = braced_body(text, line_offset + (method.start() if method else 0))
    assertions = unique_matches(
        r"\b(assert(?:Equals|NotEquals|True|False|Null|NotNull|Same|Throws|Timeout|ArrayEquals)|fail)\s*\(",
        body,
    )
    calls = unique_matches(
        r"\.(execute|check|getOutVarNames|getOutVarAttrs|getOutFunctionNames|"
        r"exportParseCache|load|addFunction|addMacro|addOperator|replaceOperator|"
        r"invoke|run|compile|parse)\s*\(",
        body,
    )
    mutations = unique_matches(
        r"\.(put|set|add|remove|clear|close|shutdown|register)\s*\(",
        body,
        limit=5,
    )
    parts = [
        "入口=" + (",".join(calls) if calls else "构造/直接调用"),
        "断言=" + (",".join(assertions) if assertions else "异常/套件内断言"),
        "副作用=" + (",".join(mutations) if mutations else "局部 fixture/无外部持久化"),
    ]
    return "；".join(parts)


def git_sha(root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "UNKNOWN"


def rust_method_evidence(rust_root: Path, item: dict) -> tuple[str, str, str]:
    """Find explicit, test-local ``Class#method`` evidence.

    The marker must be followed by a Rust ``#[test]`` function in the same
    small source window. This deliberately rejects module-level family claims.
    """
    java_class = Path(item["file"]).stem
    marker = f"{java_class}#{item['name']}"
    hits: list[tuple[str, int, str, str]] = []
    crates_root = rust_root / "crates"
    for path in sorted(crates_root.rglob("*.rs")):
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        for index, line in enumerate(lines):
            if marker not in line:
                continue
            window_start = max(0, index - 3)
            following = "\n".join(
                lines[window_start : min(len(lines), index + 18)]
            )
            test_match = re.search(
                r"#\s*\[\s*test(?:\s*\([^]]*\))?\s*\][\s\S]*?"
                r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
                following,
            )
            if not test_match:
                continue
            nearby = "\n".join(
                lines[max(0, index - 3) : min(len(lines), index + 12)]
            ).upper()
            if "PARTIAL" in nearby:
                status = "PARTIAL"
            elif "ADAPTED" in nearby:
                status = "ADAPTED"
            else:
                status = "EXACT"
            relative = path.relative_to(rust_root)
            hits.append((str(relative), index + 1, test_match.group(1), status))

    if not hits:
        return (
            "MISSING",
            "—",
            "未发现紧邻 Rust `#[test]` 的显式方法级来源标记",
        )

    statuses = {hit[3] for hit in hits}
    if "EXACT" in statuses:
        status = "EXACT"
    elif "ADAPTED" in statuses:
        status = "ADAPTED"
    else:
        status = "PARTIAL"
    evidence = "; ".join(
        f"`{path}:{line}#{test_name}`" for path, line, test_name, _ in hits
    )
    contract = {
        "EXACT": "Rust 测试声明逐项复刻 Java 输入、结果/错误与副作用",
        "ADAPTED": "存在明确平台适配，测试保留可观察契约并标注差异",
        "PARTIAL": "已有方法级测试，但来源标记明确声明尚未覆盖全部契约",
    }[status]
    return status, evidence, contract


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    inventory = json.loads(args.inventory.read_text(encoding="utf-8"))
    java_root = Path(inventory["java_root"])
    rust_root = Path(inventory["rust_root"]).resolve()
    rows = []
    for item in inventory["java_tests"]:
        status, evidence, contract = rust_method_evidence(rust_root, item)
        rows.append((item, status, evidence, contract))

    counts = Counter(status for _, status, _, _ in rows)
    java_sha = git_sha(java_root)
    rust_sha = git_sha(rust_root)
    lines = [
        "# 迁移测试对照表",
        "",
        "> 本文按 `rust-java-migration-testing` 的三台账模型记录。SOURCE_PARITY",
        "> 以 Java 测试注解为最小行，不以 Rust 测试数量或同名关系推断等价。",
        "",
        "## 固定基线与统计口径",
        "",
        f"- Java：`{java_sha}`，静态识别 **{len(rows)}** 个测试方法。",
        f"- Rust 审计基线：`{rust_sha}`；当前工作树静态识别 **{inventory['summary']['rust_tests']}** 个测试函数。",
        "- Java/Rust 覆盖率：等待本轮 JaCoCo 与 `cargo-llvm-cov` 重跑，禁止沿用旧数字。",
        f"- SOURCE_PARITY 状态：{', '.join(f'{key}={value}' for key, value in sorted(counts.items()))}。",
        "- `MISSING`/`PARTIAL` 未清零前，不得据此台账宣称迁移完成。",
        "",
        "## SOURCE_PARITY",
        "",
        "| # | Java 来源测试 | 注解 | 源方法体信号 | 迁移状态 | Rust 证据 | 已核对契约 |",
        "|---:|---|---|---|---|---|---|",
    ]
    for index, (item, status, evidence, contract) in enumerate(rows, 1):
        source = f"`{item['file']}:{item['line']}#{item['name']}`"
        lines.append(
            f"| {index} | {source} | `{item['kind']}` | {escape(source_signals(java_root, item))} | **{status}** | "
            f"{escape(evidence)} | {escape(contract)} |"
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
            "- `EXACT` 只由紧邻 Rust `#[test]` 的显式 `JavaClass#method` 来源标记产生。",
            "- `ADAPTED` 不是删减：它表示 JVM 反射、PF4J、Spring、并发对象模型或 checked exception",
            "  被 Rust 原生注册表、宿主接口、所有权和 `Result` 模型替代，外部可观察契约仍需通过。",
            "- `PARTIAL` 与 `MISSING` 是阻断状态；同名 Rust 测试、家族级映射和绿灯套件均不能自动升级。",
            "- 图级调用路径证据本轮不可用：两个仓库均无 `.codegraph/`，未擅自创建索引。",
            "",
        ]
    )
    args.output.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
