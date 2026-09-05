#!/usr/bin/env python3
"""Error-code distribution analysis for the qlexpress-rust differential corpus.

Reads ``verification/corpus/differential.jsonl`` (295 cases) and
``crates/qlexpress/src/exception/ql_error_codes.rs`` (65 error codes),
then classifies every corpus entry by expected outcome (success / error)
and maps error cases to the most likely error code.

Output:
  - ``verification/error-distribution.json``  (machine-readable)
  - ``docs/业务验收/error-distribution.md``    (human-readable report)

Usage::

    python3 scripts/analyze_error_distribution.py

No Java baseline or Rust compilation is required; the analysis is
entirely static (pattern-based classification of corpus entries).
"""

from __future__ import annotations

import json
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

# ---------------------------------------------------------------------------
# Project root (auto-detect from script location)
# ---------------------------------------------------------------------------
SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
CORPUS_PATH = PROJECT_ROOT / "verification" / "corpus" / "differential.jsonl"
ERROR_CODES_SRC = (
    PROJECT_ROOT
    / "crates"
    / "qlexpress"
    / "src"
    / "exception"
    / "ql_error_codes.rs"
)
JSON_OUTPUT = PROJECT_ROOT / "verification" / "error-distribution.json"
MD_OUTPUT = PROJECT_ROOT / "docs" / "业务验收" / "error-distribution.md"

# ---------------------------------------------------------------------------
# Error code extraction
# ---------------------------------------------------------------------------

# Regex: matches lines like `pub const FOO_BAR: &str = "FOO_BAR";`
_CODE_RE = re.compile(r'pub\s+const\s+(\w+)\s*:\s*&str\s*=\s*"(\w+)"')


def extract_error_codes(rs_path: Path) -> list[str]:
    """Extract all error-code constant values from ``ql_error_codes.rs``."""
    codes: list[str] = []
    text = rs_path.read_text(encoding="utf-8")
    for m in _CODE_RE.finditer(text):
        codes.append(m.group(2))
    return sorted(set(codes))


# ---------------------------------------------------------------------------
# Error severity classification
# ---------------------------------------------------------------------------

# P0 = compilation / sandbox / stack errors (script cannot even run)
# P1 = runtime errors (script runs but fails on specific input)
# P2 = infrastructure / cache / restriction errors

P0_CODES = {
    "SYNTAX_ERROR",
    "MISSING_INDEX",
    "INVALID_NUMBER",
    "CLASS_NOT_FOUND",
    "STACK_OVERFLOW",
    "OPERAND_STACK_OVERFLOW",
    "OPERAND_STACK_UNDERFLOW",
}

P2_CODES = {
    "OPERATOR_NOT_ALLOWED",
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION",
    "SERIALIZABLE_PARSE_CACHE_INVALID_MODEL",
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION",
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT",
    "SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND",
    "SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND",
}


def severity_of(code: str) -> str:
    if code in P0_CODES:
        return "P0"
    if code in P2_CODES:
        return "P2"
    return "P1"


# ---------------------------------------------------------------------------
# Case classification: which error code does each corpus entry trigger?
# ---------------------------------------------------------------------------

# Each classifier returns an optional error code (None = success).


def _classify_script_case(case_id: str, script: str, options: dict) -> Optional[str]:
    """Classify a ``script``-type corpus entry."""
    sid = case_id
    s = script.strip()

    # --- Syntax errors ---
    if "syntax" in sid:
        return "SYNTAX_ERROR"
    # Incomplete expression (unclosed paren, trailing operator, etc.)
    if s.endswith(("+", "-", "*", "/", "%", "(", ",", "|", "&", "^")):
        return "SYNTAX_ERROR"
    if s.count("(") > s.count(")"):
        return "SYNTAX_ERROR"

    # --- Null access ---
    # Script that is just `null` evaluates to null, but some IDs signal error
    if sid == "null-value":
        return "NULL_FIELD_ACCESS"
    if "null-field" in sid or "null-method" in sid:
        return "NULL_FIELD_ACCESS"
    if "null-call" in sid:
        return "NULL_CALL"
    if "avoid-null" in sid:
        # `missing.child.value` -- field access on undefined variable -> NULL_FIELD_ACCESS
        return "NULL_FIELD_ACCESS"
    # null in binary operators
    if "null" in sid and ("bitwise" in sid or "in-null-null" in sid or "like" in sid):
        return "INVALID_BINARY_OPERAND"
    if sid == "basebinary-in-null-list":
        return "INVALID_BINARY_OPERAND"
    if sid == "basebinary-plus-string-left-null":
        return None  # 'value=' + null => 'value=null' (string concat succeeds)

    # --- Divide / remainder by zero ---
    if "divide-by-zero" in sid or "divide-zero" in sid or "remainder-zero" in sid:
        return "INVALID_ARITHMETIC"
    if sid == "basebinary-divide-floating-zero":
        return "INVALID_ARITHMETIC"

    # --- Arithmetic overflow ---
    if "overflow" in sid:
        return "INVALID_ARITHMETIC"

    # --- Shift errors ---
    if "shift-float-left-error" in sid or "shift-float-distance-error" in sid:
        return "INVALID_BINARY_OPERAND"
    if "float" in sid and "shift" in sid:
        return "INVALID_BINARY_OPERAND"
    if sid in ("number-int-shift-distance-mask", "number-long-shift-distance-mask"):
        return None  # mask behavior, not error
    if sid in ("number-signed-right-shift", "number-unsigned-right-shift",
               "basebinary-right-shift-signed", "basebinary-right-shift-unsigned",
               "basebinary-left-shift-mask"):
        return None  # valid shift operations
    if sid == "shift":
        return None  # -1 >>> 1 = valid unsigned shift

    # --- Invalid index ---
    if "invalid-index" in sid:
        return "INVALID_INDEX"
    if "index-out-of-bound" in sid:
        return "INDEX_OUT_BOUND"

    # --- Missing function ---
    if "missing-function" in sid:
        return "FUNCTION_NOT_FOUND"

    # --- Array limit ---
    if "array-limit" in sid:
        return "EXCEED_MAX_ARR_LENGTH"

    # --- Invalid operand ---
    if "invalid-operand" in sid:
        return "INVALID_BINARY_OPERAND"
    if "invalid-left-value" in sid:
        return "INVALID_ASSIGNMENT"
    if "in-invalid-map" in sid:
        return "INVALID_BINARY_OPERAND"
    if "like-invalid" in sid:
        return "INVALID_BINARY_OPERAND"

    # --- Char multiplication ---
    if "multiply-char-error" in sid:
        return "INVALID_BINARY_OPERAND"

    # --- bigdec rounded divide ---
    if "bigdec-rounded-divide" in sid:
        return None  # precise divide with rounding

    # --- Everything else: success ---
    return None


def _classify_number_math_case(case_id: str, invocation: dict) -> Optional[str]:
    """Classify a ``number_math``-type corpus entry."""
    sid = case_id
    op = invocation.get("operation", "")
    left_type = invocation.get("left", {}).get("type", "")

    if "divide-zero" in sid:
        return "INVALID_ARITHMETIC"
    if "overflow" in sid:
        return "INVALID_ARITHMETIC"
    if "double-bitwise" in sid:
        return "INVALID_BINARY_OPERAND"
    if "float-shift-distance" in sid:
        return "INVALID_BINARY_OPERAND"
    if "bigint-unsigned-shift" in sid:
        return "INVALID_BINARY_OPERAND"
    if "nonpositive-modulus" in sid:
        return "INVALID_ARITHMETIC"
    return None


def _classify_operator_manager_case(case_id: str, invocation: dict) -> Optional[str]:
    """Classify an ``operator_manager``-type corpus entry."""
    sid = case_id
    op = invocation.get("operation", "")

    # precedence on missing operator panics -> caught as NullPointerException
    if op == "precedence" and "missing" in sid:
        return "NULL_FIELD_ACCESS"  # Rust catches panic, maps to this
    return None


def classify_case(case: dict) -> tuple[str, Optional[str]]:
    """Return ``(case_type, error_code_or_None)`` for a corpus entry."""
    case_id = case["id"]

    # Script cases
    if case.get("script") is not None:
        return "script", _classify_script_case(case_id, case["script"], case.get("options", {}))

    # NumberMath
    if case.get("number_math") is not None:
        return "number_math", _classify_number_math_case(case_id, case["number_math"])

    # OperatorManager
    if case.get("operator_manager") is not None:
        return "operator_manager", _classify_operator_manager_case(case_id, case["operator_manager"])

    # All other sub-case types (full_contract scenarios): always succeed
    for key in (
        "batch_add_function_result", "ql_functional_varargs",
        "lsp_position", "lsp_range", "lsp_diagnostic",
        "exist_stack", "macro_define", "user_define_exception",
        "security_strategies", "ql_string_utils",
        "delegate_context", "fixed_size_stack", "runtime_core",
        "exception_table",
    ):
        if case.get(key) is not None:
            return key, None

    return "unknown", None


# ---------------------------------------------------------------------------
# Severity grouping for error codes
# ---------------------------------------------------------------------------

# Group error codes by functional domain for the report.
DOMAIN_GROUPS = {
    "Syntax & Parsing": [
        "SYNTAX_ERROR", "MISSING_INDEX", "INVALID_NUMBER", "CLASS_NOT_FOUND",
    ],
    "Stack": [
        "STACK_OVERFLOW", "OPERAND_STACK_OVERFLOW", "OPERAND_STACK_UNDERFLOW",
    ],
    "Index & Access": [
        "INVALID_INDEX", "INDEX_OUT_BOUND", "NONINDEXABLE_OBJECT",
        "NONTRAVERSABLE_OBJECT", "NULL_FIELD_ACCESS", "NULL_METHOD_ACCESS",
        "FIELD_NOT_FOUND", "SET_FIELD_UNKNOWN_ERROR", "GET_FIELD_UNKNOWN_ERROR",
    ],
    "Function / Method Invocation": [
        "INVOKE_METHOD_WITH_WRONG_ARGUMENTS", "INVOKE_METHOD_INNER_ERROR",
        "INVOKE_METHOD_UNKNOWN_ERROR", "INVOKE_FUNCTION_INNER_ERROR",
        "FUNCTION_NOT_FOUND", "FUNCTION_TYPE_MISMATCH", "INVOKE_LAMBDA_ERROR",
        "NULL_CALL", "OBJECT_NOT_CALLABLE", "METHOD_NOT_FOUND",
        "INVOKE_CONSTRUCTOR_UNKNOWN_ERROR", "INVOKE_CONSTRUCTOR_INNER_ERROR",
        "NO_SUITABLE_CONSTRUCTOR",
    ],
    "Block & Control Flow": [
        "EXECUTE_BLOCK_ERROR", "FOR_EACH_ITERABLE_REQUIRED",
        "FOR_EACH_TYPE_MISMATCH", "FOR_EACH_UNKNOWN_ERROR",
        "FOR_INIT_ERROR", "FOR_BODY_ERROR", "FOR_UPDATE_ERROR",
        "FOR_CONDITION_ERROR", "FOR_CONDITION_BOOL_REQUIRED",
        "WHILE_CONDITION_BOOL_REQUIRED", "WHILE_CONDITION_ERROR",
        "CONDITION_BOOL_REQUIRED",
    ],
    "Type Cast & Assignment": [
        "INCOMPATIBLE_TYPE_CAST", "INVALID_CAST_TARGET",
        "INCOMPATIBLE_ASSIGNMENT_TYPE", "INVALID_ASSIGNMENT",
    ],
    "Arithmetic & Operators": [
        "EXECUTE_OPERATOR_EXCEPTION", "INVALID_ARITHMETIC",
        "INVALID_BINARY_OPERAND", "INVALID_UNARY_OPERAND",
    ],
    "Array": [
        "ARRAY_SIZE_NUM_REQUIRED", "EXCEED_MAX_ARR_LENGTH",
        "INCOMPATIBLE_ARRAY_ITEM_TYPE",
    ],
    "Try / Catch / Finally": [
        "EXECUTE_FINAL_BLOCK_ERROR", "EXECUTE_TRY_BLOCK_ERROR",
        "EXECUTE_CATCH_HANDLER_ERROR",
    ],
    "Timeout": [
        "SCRIPT_TIME_OUT",
    ],
    "Operator Restriction": [
        "OPERATOR_NOT_ALLOWED",
    ],
    "Serializable Parse Cache": [
        "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION",
        "SERIALIZABLE_PARSE_CACHE_INVALID_MODEL",
        "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION",
        "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT",
        "SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND",
        "SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND",
    ],
    "User Defined Exception": [
        "INVALID_ARGUMENT", "BIZ_EXCEPTION", "QL_THROW",
    ],
}


# ---------------------------------------------------------------------------
# Main analysis
# ---------------------------------------------------------------------------

@dataclass
class AnalysisResult:
    generated_at: str
    total_cases: int
    by_type: Counter = field(default_factory=Counter)
    success_count: int = 0
    error_count: int = 0
    by_error_code: Counter = field(default_factory=Counter)
    by_kind: Counter = field(default_factory=Counter)
    all_codes: list[str] = field(default_factory=list)
    covered_codes: list[str] = field(default_factory=list)
    uncovered_codes: list[str] = field(default_factory=list)
    error_details: list[dict] = field(default_factory=list)


def run_analysis() -> AnalysisResult:
    all_codes = extract_error_codes(ERROR_CODES_SRC)

    result = AnalysisResult(
        generated_at=datetime.now(timezone.utc).isoformat(),
        total_cases=0,
        all_codes=all_codes,
    )

    # Read corpus
    with open(CORPUS_PATH, encoding="utf-8") as f:
        for line_no, line in enumerate(f, 1):
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            case = json.loads(line)
            result.total_cases += 1

            case_type, error_code = classify_case(case)
            result.by_type[case_type] += 1

            if error_code is None:
                result.success_count += 1
                result.by_kind["Success"] += 1
            else:
                result.error_count += 1
                result.by_error_code[error_code] += 1
                result.by_kind[severity_of(error_code)] += 1
                result.error_details.append({
                    "id": case["id"],
                    "case_type": case_type,
                    "error_code": error_code,
                    "severity": severity_of(error_code),
                })

    # Ensure all known codes appear in by_error_code (even if 0)
    for code in all_codes:
        if code not in result.by_error_code:
            result.by_error_code[code] = 0

    result.covered_codes = [c for c in all_codes if result.by_error_code.get(c, 0) > 0]
    result.uncovered_codes = [c for c in all_codes if result.by_error_code.get(c, 0) == 0]

    return result


# ---------------------------------------------------------------------------
# JSON output
# ---------------------------------------------------------------------------

def write_json(result: AnalysisResult, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = {
        "generated_at": result.generated_at,
        "total_cases": result.total_cases,
        "executed": result.total_cases,
        "success_count": result.success_count,
        "exception_count": result.error_count,
        "by_error_code": dict(sorted(result.by_error_code.items())),
        "by_kind": dict(result.by_kind.most_common()),
        "covered_codes": result.covered_codes,
        "uncovered_codes": result.uncovered_codes,
        "error_details": result.error_details,
    }
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")


# ---------------------------------------------------------------------------
# Markdown report
# ---------------------------------------------------------------------------

def write_markdown(result: AnalysisResult, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []
    w = lines.append

    w("# QLExpress-Rust 错误码分布报告")
    w("")
    w(f"> 生成时间: {result.generated_at}")
    w(f"> 语料来源: `verification/corpus/differential.jsonl` ({result.total_cases} 条)")
    w(f"> 错误码来源: `crates/qlexpress/src/exception/ql_error_codes.rs` ({len(result.all_codes)} 个)")
    w("")

    # --- Summary ---
    w("## 1. 总览")
    w("")
    w(f"| 指标 | 值 |")
    w(f"|---|---|")
    w(f"| 语料总数 | {result.total_cases} |")
    w(f"| 预期成功 | {result.success_count} |")
    w(f"| 预期异常 | {result.error_count} |")
    w(f"| 触发的错误码种类 | {len(result.covered_codes)} / {len(result.all_codes)} |")
    w(f"| 未触发的错误码种类 | {len(result.uncovered_codes)} |")
    w("")

    # --- By kind ---
    w("## 2. 按严重度分布")
    w("")
    w("| 严重度 | 数量 | 占异常比 |")
    w("|---|---|---|")
    for kind in ["P0", "P1", "P2", "Success"]:
        count = result.by_kind.get(kind, 0)
        if kind == "Success":
            pct = f"{count / result.total_cases * 100:.1f}% (占总数)"
        elif result.error_count > 0:
            pct = f"{count / result.error_count * 100:.1f}%"
        else:
            pct = "0.0%"
        w(f"| {kind} | {count} | {pct} |")
    w("")

    # --- By case type ---
    w("## 3. 按用例类型分布")
    w("")
    w("| 用例类型 | 数量 |")
    w("|---|---|")
    for case_type, count in result.by_type.most_common():
        w(f"| {case_type} | {count} |")
    w("")

    # --- Error code table ---
    w("## 4. 错误码触发明细")
    w("")
    w("| 错误码 | 触发次数 | 占异常比 | 严重度 |")
    w("|---|---|---|---|")

    # Sort by count descending, then by code name
    sorted_codes = sorted(
        result.by_error_code.items(),
        key=lambda kv: (-kv[1], kv[0]),
    )
    for code, count in sorted_codes:
        if count > 0:
            pct = f"{count / result.error_count * 100:.1f}%" if result.error_count > 0 else "0.0%"
            sev = severity_of(code)
            w(f"| `{code}` | {count} | {pct} | {sev} |")
    w("")

    # --- Uncovered codes ---
    w("## 5. 未触发的错误码 (仓库语料未覆盖)")
    w("")
    if result.uncovered_codes:
        w("以下错误码在 295 条差分语料中**未被任何用例触发**：")
        w("")
        # Group by domain
        uncovered_set = set(result.uncovered_codes)
        for domain, codes in DOMAIN_GROUPS.items():
            domain_uncovered = [c for c in codes if c in uncovered_set]
            if domain_uncovered:
                w(f"**{domain}**:")
                w("")
                for code in domain_uncovered:
                    w(f"- `{code}` ({severity_of(code)})")
                w("")
    else:
        w("所有错误码均已被语料覆盖。")
        w("")

    # --- Domain coverage summary ---
    w("## 6. 按功能域覆盖情况")
    w("")
    w("| 功能域 | 已覆盖 | 未覆盖 | 覆盖率 |")
    w("|---|---|---|---|")
    covered_set = set(result.covered_codes)
    for domain, codes in DOMAIN_GROUPS.items():
        dom_covered = sum(1 for c in codes if c in covered_set)
        dom_total = len(codes)
        pct = f"{dom_covered / dom_total * 100:.0f}%" if dom_total > 0 else "N/A"
        w(f"| {domain} | {dom_covered} | {dom_total - dom_covered} | {pct} |")
    w("")

    # --- Error details ---
    w("## 7. 异常用例明细")
    w("")
    w("| 用例 ID | 用例类型 | 错误码 | 严重度 |")
    w("|---|---|---|---|")
    for detail in result.error_details:
        w(f"| `{detail['id']}` | {detail['case_type']} | `{detail['error_code']}` | {detail['severity']} |")
    w("")

    # --- Insights ---
    w("## 8. 对业务验收的启示")
    w("")
    w("### 高频错误码需单独验证")
    w("")
    w("以下错误码在仓库语料中被高频触发，业务脚本应额外覆盖其边界条件：")
    w("")
    high_freq = [(c, n) for c, n in sorted_codes if n > 0]
    for code, count in high_freq:
        w(f"- **`{code}`** (触发 {count} 次) -- 需在业务脚本中验证不同输入模式下的触发路径")
    w("")

    w("### 未覆盖错误码需补充验证")
    w("")
    uncovered_by_p0 = [c for c in result.uncovered_codes if severity_of(c) == "P0"]
    uncovered_by_p1 = [c for c in result.uncovered_codes if severity_of(c) == "P1"]
    uncovered_by_p2 = [c for c in result.uncovered_codes if severity_of(c) == "P2"]

    if uncovered_by_p0:
        w("**P0 (编译/沙箱/栈) 未覆盖** -- 这些是编译期或基础设施错误，")
        w("业务脚本需要专门构造触发条件：")
        w("")
        for code in uncovered_by_p0:
            w(f"- `{code}`")
        w("")

    if uncovered_by_p1:
        w("**P1 (运行时) 未覆盖** -- 这些是运行时错误，")
        w("业务脚本应通过构造特定输入来覆盖：")
        w("")
        for code in uncovered_by_p1:
            w(f"- `{code}`")
        w("")

    if uncovered_by_p2:
        w("**P2 (基础设施/缓存) 未覆盖** -- 这些错误涉及序列化缓存和操作符限制，")
        w("可在集成测试阶段覆盖：")
        w("")
        for code in uncovered_by_p2:
            w(f"- `{code}`")
        w("")

    w("### 验收建议")
    w("")
    w("1. **优先补充 P0 未覆盖错误码**：编译期错误对用户体验影响最大")
    w("2. **高频 P1 错误码需多角度验证**：同一错误码的不同触发路径可能暴露不同的 bug")
    w("3. **P2 错误码可在集成测试中覆盖**：序列化缓存等场景适合端到端测试")
    w("4. **成功率基线**：当前语料预期成功率 "
      f"{result.success_count / result.total_cases * 100:.1f}%，"
      f"异常率 {result.error_count / result.total_cases * 100:.1f}%")
    w("")

    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main() -> None:
    print(f"Reading corpus: {CORPUS_PATH}")
    print(f"Reading error codes: {ERROR_CODES_SRC}")

    result = run_analysis()

    write_json(result, JSON_OUTPUT)
    print(f"JSON output: {JSON_OUTPUT}")

    write_markdown(result, MD_OUTPUT)
    print(f"Markdown output: {MD_OUTPUT}")

    print(f"\n--- Summary ---")
    print(f"Total cases: {result.total_cases}")
    print(f"Expected success: {result.success_count}")
    print(f"Expected error:   {result.error_count}")
    print(f"Error codes triggered: {len(result.covered_codes)} / {len(result.all_codes)}")
    print(f"Error codes uncovered: {len(result.uncovered_codes)}")
    print(f"\nBy error code (non-zero):")
    for code, count in sorted(result.by_error_code.items(), key=lambda kv: -kv[1]):
        if count > 0:
            print(f"  {code}: {count}")


if __name__ == "__main__":
    main()
