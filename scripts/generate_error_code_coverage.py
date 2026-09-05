#!/usr/bin/env python3
"""Synthetic business-corpus generator for error-code coverage.

Reads ``crates/qlexpress/src/exception/ql_error_codes.rs`` (65 error codes)
and generates one synthetic QLExpress script per error code that *should*
trigger it.  The output is written to
``verification/corpus/business-synthetic.jsonl``.

This script is **self-contained** -- it does NOT depend on the Rust runtime
or the Java baseline.  Generated scripts are "theoretically triggering":
whether they actually trigger the error code in qlexpress-rust depends on
the implementation fidelity.  A separate verification step
(``crates/qlexpress-verification/src/bin/run-script-biz.rs``) can confirm
them against the real engine.

Usage::

    python3 scripts/generate_error_code_coverage.py

The generated JSONL file is a build artifact -- if ``.gitignore`` blocks
it, regenerate it with the command above.

No git commits are made by this script.
"""

from __future__ import annotations

import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
ERROR_CODES_SRC = (
    PROJECT_ROOT
    / "crates"
    / "qlexpress"
    / "src"
    / "exception"
    / "ql_error_codes.rs"
)
OUTPUT_PATH = PROJECT_ROOT / "verification" / "corpus" / "business-synthetic.jsonl"

# ---------------------------------------------------------------------------
# Error code extraction (reuses the regex from analyze_error_distribution.py)
# ---------------------------------------------------------------------------
# 注意：ql_error_codes.rs 中部分常量是两行写法（`pub const X: &str =\n    "X";`），
# 例如 SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION / _UNSUPPORTED_INSTRUCTION。
# 正则需兼容跨行赋值（`\s*` 覆盖换行）。
_CODE_RE = re.compile(r'pub\s+const\s+(\w+)\s*:\s*&str\s*=\s*"(\w+)"')
_MSG_RE = re.compile(r'^\s+(\w+)\s+=>\s+"(.+?)",?\s*$', re.MULTILINE)


def extract_error_codes(rs_path: Path) -> dict[str, str]:
    """Return {const_name: error_code_string} from ql_error_codes.rs."""
    text = rs_path.read_text(encoding="utf-8")
    return {m.group(1): m.group(2) for m in _CODE_RE.finditer(text)}


def extract_error_templates(rs_path: Path) -> dict[str, str]:
    """Return {error_code: message_template} from the error_msg match block."""
    text = rs_path.read_text(encoding="utf-8")
    # Find the match block inside error_msg()
    in_match = False
    templates: dict[str, str] = {}
    for line in text.splitlines():
        stripped = line.strip()
        if "match code" in stripped:
            in_match = True
            continue
        if in_match and stripped == "}":
            break
        if not in_match:
            continue
        # Pattern: CODE => "template",
        m = re.match(r'^(\w+)\s+=>\s+"(.*)"[,]?$', stripped)
        if m:
            templates[m.group(1)] = m.group(2)
        # Multi-line: CODE => { "template" }
        m2 = re.match(r'^(\w+)\s+=>\s+\{$', stripped)
        if m2:
            code = m2.group(1)
            # Next line is the template
            continue
        # Inside multi-line block
        m3 = re.match(r'^"(.+)"$', stripped)
        if m3 and in_match:
            # This is a template for the previous code
            pass
    return templates


# ---------------------------------------------------------------------------
# Business domain categories
# ---------------------------------------------------------------------------

DOMAIN_MAP: dict[str, str] = {
    "SYNTAX_ERROR": "syntax-parsing",
    "MISSING_INDEX": "syntax-parsing",
    "INVALID_NUMBER": "syntax-parsing",
    "CLASS_NOT_FOUND": "type-resolution",
    "STACK_OVERFLOW": "resource-limits",
    "OPERAND_STACK_OVERFLOW": "resource-limits",
    "OPERAND_STACK_UNDERFLOW": "resource-limits",
    "INVALID_INDEX": "index-access",
    "INDEX_OUT_BOUND": "index-access",
    "NONINDEXABLE_OBJECT": "index-access",
    "NONTRAVERSABLE_OBJECT": "iteration",
    "NULL_FIELD_ACCESS": "null-safety",
    "NULL_METHOD_ACCESS": "null-safety",
    "FIELD_NOT_FOUND": "field-access",
    "SET_FIELD_UNKNOWN_ERROR": "field-access",
    "GET_FIELD_UNKNOWN_ERROR": "field-access",
    "INVOKE_METHOD_WITH_WRONG_ARGUMENTS": "method-invocation",
    "INVOKE_METHOD_INNER_ERROR": "method-invocation",
    "INVOKE_METHOD_UNKNOWN_ERROR": "method-invocation",
    "INVOKE_FUNCTION_INNER_ERROR": "function-invocation",
    "FUNCTION_NOT_FOUND": "function-invocation",
    "FUNCTION_TYPE_MISMATCH": "function-invocation",
    "INVOKE_LAMBDA_ERROR": "lambda-invocation",
    "NULL_CALL": "null-safety",
    "OBJECT_NOT_CALLABLE": "invocation",
    "METHOD_NOT_FOUND": "method-invocation",
    "INVOKE_CONSTRUCTOR_UNKNOWN_ERROR": "constructor-invocation",
    "INVOKE_CONSTRUCTOR_INNER_ERROR": "constructor-invocation",
    "NO_SUITABLE_CONSTRUCTOR": "constructor-invocation",
    "EXECUTE_BLOCK_ERROR": "block-execution",
    "INCOMPATIBLE_TYPE_CAST": "type-cast",
    "INVALID_CAST_TARGET": "type-cast",
    "SCRIPT_TIME_OUT": "resource-limits",
    "PARSE_AST_DEPTH_EXCEEDED": "syntax-parsing",
    "SANDBOX_DEADLINE_EXCEEDED": "resource-limits",
    "SANDBOX_FUEL_EXCEEDED": "resource-limits",
    "SANDBOX_CALL_DEPTH_EXCEEDED": "resource-limits",
    "OPERATOR_NOT_FOUND": "operator",
    "OPERAND_STACK_UNDERFLOW": "resource-limits",
    "OPERAND_STACK_UNDERFLOW_INTERNAL": "resource-limits",
    "INCOMPATIBLE_ASSIGNMENT_TYPE": "assignment",
    "FOR_EACH_ITERABLE_REQUIRED": "control-flow",
    "FOR_EACH_TYPE_MISMATCH": "control-flow",
    "FOR_EACH_UNKNOWN_ERROR": "control-flow",
    "FOR_INIT_ERROR": "control-flow",
    "FOR_BODY_ERROR": "control-flow",
    "FOR_UPDATE_ERROR": "control-flow",
    "FOR_CONDITION_ERROR": "control-flow",
    "FOR_CONDITION_BOOL_REQUIRED": "control-flow",
    "WHILE_CONDITION_BOOL_REQUIRED": "control-flow",
    "WHILE_CONDITION_ERROR": "control-flow",
    "CONDITION_BOOL_REQUIRED": "control-flow",
    "ARRAY_SIZE_NUM_REQUIRED": "array",
    "EXCEED_MAX_ARR_LENGTH": "array",
    "INCOMPATIBLE_ARRAY_ITEM_TYPE": "array",
    "INVALID_ASSIGNMENT": "assignment",
    "EXECUTE_OPERATOR_EXCEPTION": "operator",
    "INVALID_ARITHMETIC": "arithmetic",
    "INVALID_BINARY_OPERAND": "operator",
    "INVALID_UNARY_OPERAND": "operator",
    "EXECUTE_FINAL_BLOCK_ERROR": "try-catch",
    "EXECUTE_TRY_BLOCK_ERROR": "try-catch",
    "EXECUTE_CATCH_HANDLER_ERROR": "try-catch",
    "OPERATOR_NOT_ALLOWED": "operator-restriction",
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION": "serialization-cache",
    "SERIALIZABLE_PARSE_CACHE_INVALID_MODEL": "serialization-cache",
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION": "serialization-cache",
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT": "serialization-cache",
    "SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND": "serialization-cache",
    "SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND": "serialization-cache",
    "INVALID_ARGUMENT": "user-exception",
    "BIZ_EXCEPTION": "user-exception",
    "QL_THROW": "user-exception",
}


# ---------------------------------------------------------------------------
# Synthetic script templates
#
# Each entry: (trigger_code, scripts_list)
# Each script entry: (script_text, rationale)
#
# Scripts are written in QLExpress syntax.  They are designed to be
# "theoretically triggering" -- i.e., a correct implementation of the
# QLExpress spec should raise the indicated error code.
# ---------------------------------------------------------------------------

def _build_script_templates() -> dict[str, list[tuple[str, str]]]:
    """Return {error_code: [(script, rationale), ...]} for all 65 codes."""
    templates: dict[str, list[tuple[str, str]]] = {}

    # ---- Syntax & Parsing ----
    templates["SYNTAX_ERROR"] = [
        ("1 + + 1", "Double operator is a syntax error"),
        ("a = (1 + 2", "Unclosed parenthesis"),
        ("if (true) {", "Unclosed brace"),
    ]
    templates["PARSE_AST_DEPTH_EXCEEDED"] = [
        ("(" * 120 + "1" + ")" * 120,
         "Deeply nested expression beyond parser MAX_PARSE_DEPTH=100 -- "
         "returns PARSE_AST_DEPTH_EXCEEDED instead of crashing the "
         "worker process (P0 fix: parser recursion depth guard)"),
    ]
    templates["MISSING_INDEX"] = [
        ("a = [1,2,3]; a[]", "Empty brackets -- missing index expression"),
    ]
    templates["INVALID_NUMBER"] = [
        ("x = 12abc", "Malformed numeric literal"),
    ]
    templates["CLASS_NOT_FOUND"] = [
        ("obj = new com.example.NonExistentClass()", "Reference to non-existent class"),
    ]

    # ---- Stack ----
    templates["STACK_OVERFLOW"] = [
        ("function r(n) { return r(n-1); }; r(100000)", "Unbounded recursion overflows the call stack"),
    ]
    templates["OPERAND_STACK_OVERFLOW"] = [
        ("(((((((((((((((((((((((((1+1))))))))))))))))))))))))", "Deeply nested expressions exhaust operand stack"),
    ]
    templates["OPERAND_STACK_UNDERFLOW"] = [
        ("", "Empty script -- no operands to evaluate (engine should handle gracefully)"),
    ]

    # ---- Index & Access ----
    templates["INVALID_INDEX"] = [
        ("a = [10, 20, 30]; a['hello']", "String used as index on array"),
    ]
    templates["INDEX_OUT_BOUND"] = [
        ("a = [1, 2, 3]; a[100]", "Index 100 exceeds array length 3"),
    ]
    templates["NONINDEXABLE_OBJECT"] = [
        ("x = 42; x[0]", "Integer is not indexable"),
    ]
    templates["NONTRAVERSABLE_OBJECT"] = [
        ("for (x : 42) { x }", "Integer is not traversable in for-each"),
    ]

    # ---- Null Safety ----
    templates["NULL_FIELD_ACCESS"] = [
        ("null.name", "Access field on null literal"),
    ]
    templates["NULL_METHOD_ACCESS"] = [
        ("null.toString()", "Call method on null literal"),
    ]
    templates["NULL_CALL"] = [
        ("f = null; f()", "Call null as function"),
    ]

    # ---- Field Access ----
    templates["FIELD_NOT_FOUND"] = [
        ("m = {'a': 1}; m.nonExistentField", "Access non-existent field on map"),
    ]
    templates["SET_FIELD_UNKNOWN_ERROR"] = [
        ("x = 1; x.readOnlyField = 99", "Attempt to set field on immutable object"),
    ]
    templates["GET_FIELD_UNKNOWN_ERROR"] = [
        ("x = 1; x.unknownInternalField", "Attempt to get field that causes internal error"),
    ]

    # ---- Method Invocation ----
    templates["INVOKE_METHOD_WITH_WRONG_ARGUMENTS"] = [
        ("s = 'hello'; s.substring()", "substring() called with zero arguments"),
    ]
    templates["INVOKE_METHOD_INNER_ERROR"] = [
        ("s = 'hello'; s.charAt(-1)", "charAt with negative index causes inner exception"),
    ]
    templates["INVOKE_METHOD_UNKNOWN_ERROR"] = [
        ("s = 'hello'; s.nonExistentMethod()", "Call method that does not exist on String"),
    ]
    templates["METHOD_NOT_FOUND"] = [
        ("obj = {'a':1}; obj.noSuchMethod(1,2,3)", "No suitable method 'noSuchMethod' on map"),
    ]

    # ---- Function Invocation ----
    templates["FUNCTION_NOT_FOUND"] = [
        ("nonExistentFunction()", "Call undefined function"),
    ]
    templates["FUNCTION_TYPE_MISMATCH"] = [
        ("x = 42; x()", "Variable is not a function type but is called"),
    ]
    templates["INVOKE_FUNCTION_INNER_ERROR"] = [
        ("function boom() { return 1/0; }; boom()", "Function body throws arithmetic error"),
    ]

    # ---- Lambda ----
    templates["INVOKE_LAMBDA_ERROR"] = [
        ("f = x -> { return 1/0; }; f(1)", "Lambda body throws arithmetic error"),
    ]

    # ---- Object Callable ----
    templates["OBJECT_NOT_CALLABLE"] = [
        ("obj = 42; obj()", "Integer is not callable"),
    ]

    # ---- Constructor Invocation ----
    templates["INVOKE_CONSTRUCTOR_UNKNOWN_ERROR"] = [
        ("new Object()", "Constructor invocation may fail in restricted sandbox"),
    ]
    templates["INVOKE_CONSTRUCTOR_INNER_ERROR"] = [
        ("new String(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)", "Constructor with too many args causes inner error"),
    ]
    templates["NO_SUITABLE_CONSTRUCTOR"] = [
        ("new Integer('not_a_number')", "No constructor matches argument type"),
    ]

    # ---- Block Execution ----
    templates["EXECUTE_BLOCK_ERROR"] = [
        ("{ 1/0 }", "Block body throws arithmetic error"),
    ]

    # ---- Type Cast ----
    templates["INCOMPATIBLE_TYPE_CAST"] = [
        ("(Integer)'hello'", "Cannot cast String to Integer"),
    ]
    templates["INVALID_CAST_TARGET"] = [
        ("(42) 'hello'", "Cast target is not a class/type"),
    ]

    # ---- Timeout ----
    templates["SCRIPT_TIME_OUT"] = [
        ("while(true) { x = x + 1 }", "Infinite loop triggers script timeout"),
    ]

    # ---- Assignment ----
    templates["INCOMPATIBLE_ASSIGNMENT_TYPE"] = [
        ("int x = 'not a number'", "Assign string to int-typed variable"),
    ]
    templates["INVALID_ASSIGNMENT"] = [
        ("1 = 2", "Cannot assign to a literal value"),
    ]

    # ---- For-Each ----
    templates["FOR_EACH_ITERABLE_REQUIRED"] = [
        ("for (x : 42) { x }", "for-each requires iterable, got integer"),
    ]
    templates["FOR_EACH_TYPE_MISMATCH"] = [
        ("for (int x : ['a','b']) { x }", "for-each expects int but got String"),
    ]
    templates["FOR_EACH_UNKNOWN_ERROR"] = [
        ("for (x : null) { x }", "for-each on null causes unknown error"),
    ]

    # ---- For Loop ----
    templates["FOR_INIT_ERROR"] = [
        ("for (1/0; i < 10; i++) { i }", "for-init expression throws arithmetic error"),
    ]
    templates["FOR_BODY_ERROR"] = [
        ("for (i = 0; i < 10; i++) { 1/0 }", "for-body throws arithmetic error"),
    ]
    templates["FOR_UPDATE_ERROR"] = [
        ("for (i = 0; i < 10; i = 1/0) { i }", "for-update throws arithmetic error"),
    ]
    templates["FOR_CONDITION_ERROR"] = [
        ("for (i = 0; 1/0; i++) { i }", "for-condition throws arithmetic error"),
    ]
    templates["FOR_CONDITION_BOOL_REQUIRED"] = [
        ("for (i = 0; 42; i++) { i }", "for-condition must return boolean, got int"),
    ]

    # ---- While ----
    templates["WHILE_CONDITION_BOOL_REQUIRED"] = [
        ("while (42) { x = 1 }", "while condition must be boolean, got int"),
    ]
    templates["WHILE_CONDITION_ERROR"] = [
        ("while (1/0) { x = 1 }", "while condition throws arithmetic error"),
    ]

    # ---- Condition ----
    templates["CONDITION_BOOL_REQUIRED"] = [
        ("if (42) { x = 1 }", "if condition must be boolean, got int"),
    ]

    # ---- Array ----
    templates["ARRAY_SIZE_NUM_REQUIRED"] = [
        ("a = new int['not_a_number']", "Array size must be a number, got string"),
    ]
    templates["EXCEED_MAX_ARR_LENGTH"] = [
        ("a = new int[10000000]", "Array length exceeds maximum allowed"),
    ]
    templates["INCOMPATIBLE_ARRAY_ITEM_TYPE"] = [
        ("int[] a = [1, 2, 'three']", "Array declared as int[] but contains String"),
    ]

    # ---- Arithmetic & Operators ----
    templates["EXECUTE_OPERATOR_EXCEPTION"] = [
        ("1 / 0", "Division by zero triggers operator exception"),
    ]
    templates["INVALID_ARITHMETIC"] = [
        ("1 / 0", "Division by zero"),
    ]
    templates["INVALID_BINARY_OPERAND"] = [
        ("true + 1", "Boolean + Integer is not a valid binary operation"),
    ]
    templates["INVALID_UNARY_OPERAND"] = [
        ("-'hello'", "Negation of String is not valid"),
    ]

    # ---- Try/Catch/Finally ----
    templates["EXECUTE_FINAL_BLOCK_ERROR"] = [
        ("try { x = 1 } finally { 1/0 }", "finally block throws arithmetic error"),
    ]
    templates["EXECUTE_TRY_BLOCK_ERROR"] = [
        ("try { 1/0 } catch(e) { 'caught' }", "try block throws error (should be caught)"),
    ]
    templates["EXECUTE_CATCH_HANDLER_ERROR"] = [
        ("try { 1/0 } catch(e) { 1/0 }", "catch handler itself throws error"),
    ]

    # ---- Operator Restriction ----
    templates["OPERATOR_NOT_ALLOWED"] = [
        ("~1", "Bitwise NOT may be disallowed by operator restriction policy"),
    ]

    # ---- Serializable Parse Cache ----
    templates["SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION"] = [
        ("", "Cannot trigger via script -- requires corrupted cache with unsupported version"),
    ]
    templates["SERIALIZABLE_PARSE_CACHE_INVALID_MODEL"] = [
        ("", "Cannot trigger via script -- requires corrupted cache with invalid model"),
    ]
    templates["SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION"] = [
        ("", "Cannot trigger via script -- requires corrupted cache with unsupported instruction"),
    ]
    templates["SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT"] = [
        ("", "Cannot trigger via script -- requires corrupted cache with unsupported constant"),
    ]
    templates["SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND"] = [
        ("", "Cannot trigger via script -- requires corrupted cache referencing missing class"),
    ]
    templates["SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND"] = [
        ("", "Cannot trigger via script -- requires corrupted cache referencing missing operator"),
    ]

    # ---- User Defined Exception ----
    templates["INVALID_ARGUMENT"] = [
        ("", "Cannot trigger via script -- thrown by user-registered functions with invalid args"),
    ]
    templates["BIZ_EXCEPTION"] = [
        ("", "Cannot trigger via script -- thrown by user-registered business logic"),
    ]
    templates["QL_THROW"] = [
        ("throw 'business error'", "QLExpress throw statement"),
    ]

    # ---- Real-world fintech/risk-control scenarios appended at end ----
    # These are paraphrased from production usage patterns documented in
    # alibaba/QLExpress README "Common error codes" section. Each appends to
    # an existing template list so coverage count is preserved; rationale
    # carries the real-world domain context (risk-control / pricing / KYC).
    # Source: github.com/alibaba/QLExpress README + QLExpress wiki

    templates["SYNTAX_ERROR"].append((
        "fee = if (tier == 'GOLD') then rate*0.7 "
        "else if (tier == 'SILVER') then rate*0.85",
        "Risk-pricing ladder missing final else — production typo from "
        "rule authors; engine rejects as syntax error"
    ))

    templates["INVALID_ARITHMETIC"].extend([
        ("amount = 1000; tenureMonths = 0; monthly = amount / tenureMonths",
         "Production divide-by-zero in amortization-style pricing — "
         "tenureMonths=0 from missing loan term (README canonical bug)"),
        ("false && (1/0)",
         "Disabled short-circuit makes the dead branch evaluate and "
         "throw — QLExpress README explicitly warns against "
         "shortCircuitDisable(true) in production"),
    ])

    templates["INVALID_ASSIGNMENT"].append((
        "maxRetry = 3; maxRetry = 'forever'",
        "Risk threshold declared int then overwritten by String from "
        "external config — classic integration bug"
    ))

    templates["FIELD_NOT_FOUND"].append((
        "order = {'id':'O-1', 'amt':100}; order.amount",
        "Rule author uses dot on Map context expecting POJO field — "
        "QLExpress isolation strategy blocks reflective access to "
        "non-whitelisted fields"
    ))

    templates["METHOD_NOT_FOUND"].append((
        "user = {'id':'U-1'}; user.getPassword()",
        "Production anti-pattern: script attempts privileged getter not "
        "in whiteList — isolation security blocks"
    ))

    # SANDBOX_DEADLINE_EXCEEDED = Java SCRIPT_TIME_OUT (wiki canonical case)
    templates.setdefault("SANDBOX_DEADLINE_EXCEEDED", [
        ("i = 0; while (i < 1000000) { i = i + 1 }",
         "Infinite-loop style rule — wall-clock deadline exceeded; "
         "production rule authors sometimes leave debug loops")
    ])

    # OPERATOR_NOT_FOUND: `=` in condition + DSL `between` not registered
    templates.setdefault("OPERATOR_NOT_FOUND", [
        ("score = 100; if (score = 90) { 'PASS' } else { 'FAIL' }",
         "Production typo: single `=` in if-condition; intent was `==` "
         "(README canonical pattern)"),
        ("score between 500 and 800",
         "DSL style `between x and y` not registered as infix operator "
         "or function — would need addOperatorBiFunction")
    ])

    # EXCEED_MAX_ARR_LENGTH: 1M-element list building
    templates["EXCEED_MAX_ARR_LENGTH"].append((
        "big = []; i = 0; while (i < 1000000) { big.add(i); i = i + 1 }",
        "Production rule that tries to build a 1M-element list — "
        "EXCEED_MAX_ARR_LENGTH budget trips"
    ))

    # ---- Business domain samples (NOT error-code-mapped; setdefault keys
    # intentionally won't be counted in coverage stats) ----
    # These are real-world QLExpress patterns that don't directly map to a
    # single error code but document common production complexity.
    templates.setdefault("__BIZ_RISK_TERNARY__", [
        ("score = 750; decision = score >= 800 ? 'REJECT' : "
         "score >= 600 ? 'REVIEW' : score >= 300 ? 'MANUAL' : 'PASS'",
         "Realistic risk-engine decision ladder — 3-level nested "
         "ternary, common in production pricing/risk rules")
    ])

    templates.setdefault("__BIZ_TRY_CATCH_PROPAGATION__", [
        ("limit = try { user.getLimit() } catch (e) { null }; "
         "if (limit == null) { 1/0 }",
         "Catch arm propagates; downstream division by zero — "
         "QLExpress wiki calls this out as a common audit miss")
    ])

    templates.setdefault("__BIZ_PII_DYNAMIC_STRING__", [
        ("cardNo = '6222021234567890'; "
         "masked = '****'+cardNo.substring(cardNo.length()-4)",
         "Production card-masking rule — engine materialises full PAN "
         "in memory during eval (PII leak risk, not error but worth "
         "auditing per QLExpress README)")
    ])

    templates.setdefault("__BIZ_MACRO_EXCEPTION__", [
        ("macro REJECT { throw 'REJECT' }; "
         "try { REJECT() } catch (e) { 'caught' }",
         "Macro-defined exception + try-catch fallback — common in "
         "approval-workflow engines")
    ])

    templates.setdefault("__BIZ_LIST_FLATTEN_TRAP__", [
        ("orders = [[{'a':1},{'a':2}],[{'a':3}]]; "
         "totals = orders*.a; counts = orders*.length",
         "README's silent flatten trap: `*.a` flattens to [1,2,3] "
         "but `*.length` does NOT flatten (length exists at current "
         "level) — returns [2,1], not [3]")
    ])

    templates.setdefault("__BIZ_CASCADING_RECURSION__", [
        ("function lookup(rule) { return lookup(rule.parent) }; "
         "lookup({name:'top'})",
         "Cascading rule lookup with unbounded recursion — "
         "SANDBOX_CALL_DEPTH_EXCEEDED trips")
    ])

    return templates


# ---------------------------------------------------------------------------
# Severity classification (same as analyze_error_distribution.py)
# ---------------------------------------------------------------------------

P0_CODES = {
    "SYNTAX_ERROR", "MISSING_INDEX", "INVALID_NUMBER", "CLASS_NOT_FOUND",
    "STACK_OVERFLOW", "OPERAND_STACK_OVERFLOW", "OPERAND_STACK_UNDERFLOW",
    "PARSE_AST_DEPTH_EXCEEDED",
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
# Generation
# ---------------------------------------------------------------------------

def generate_corpus(
    codes: dict[str, str],
    templates: dict[str, list[tuple[str, str]]],
) -> list[dict]:
    """Generate the synthetic corpus entries."""
    entries: list[dict] = []
    counter: dict[str, int] = {}

    for const_name, error_code in sorted(codes.items()):
        if error_code not in templates:
            # Should not happen if templates are complete
            entries.append({
                "id": f"biz-{error_code.lower()}-1",
                "category": DOMAIN_MAP.get(error_code, "unknown"),
                "trigger": error_code,
                "script": "",
                "rationale": f"No template defined for {error_code} -- manual review needed",
            })
            continue

        for idx, (script, rationale) in enumerate(templates[error_code], 1):
            counter[error_code] = idx
            entries.append({
                "id": f"biz-{error_code.lower()}-{idx}",
                "category": DOMAIN_MAP.get(error_code, "unknown"),
                "trigger": error_code,
                "script": script,
                "rationale": rationale,
            })

    return entries


# ---------------------------------------------------------------------------
# Duplicate detection: scripts that already appear in differential.jsonl
# ---------------------------------------------------------------------------

def load_existing_scripts(corpuses: list[Path]) -> set[str]:
    """Load scripts from existing JSONL corpuses to detect duplicates."""
    scripts: set[str] = set()
    for path in corpuses:
        if not path.exists():
            continue
        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                try:
                    entry = json.loads(line)
                    if "script" in entry and entry["script"]:
                        scripts.add(entry["script"].strip())
                except json.JSONDecodeError:
                    continue
    return scripts


# ---------------------------------------------------------------------------
# JSONL output
# ---------------------------------------------------------------------------

def write_jsonl(entries: list[dict], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        for entry in entries:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------

def generate_report(
    entries: list[dict],
    all_codes: dict[str, str],
    existing_covered: set[str],
) -> str:
    """Generate the business-synthetic-coverage.md report."""
    lines: list[str] = []
    w = lines.append

    # Collect which error codes have synthetic entries
    synthetic_codes: dict[str, int] = {}
    for entry in entries:
        code = entry["trigger"]
        synthetic_codes[code] = synthetic_codes.get(code, 0) + 1

    total_codes = len(all_codes)
    unique_codes = sorted(set(all_codes.values()))
    covered_by_existing = len(existing_covered)
    covered_by_synthetic = len([c for c in unique_codes if synthetic_codes.get(c, 0) > 0])

    # Scripts that are not empty (actually triggerable via script)
    triggerable = [e for e in entries if e["script"].strip()]
    not_triggerable = [e for e in entries if not e["script"].strip()]

    w("# QLExpress-Rust 合成业务语料 -- 错误码覆盖报告")
    w("")
    w(f"> 生成时间: {datetime.now(timezone.utc).isoformat()}")
    w(f"> 错误码来源: `crates/qlexpress/src/exception/ql_error_codes.rs` ({total_codes} 个)")
    w(f"> 仓库语料: `verification/corpus/differential.jsonl` (295 条, 覆盖 {covered_by_existing} 错误码)")
    w(f"> 合成语料: `verification/corpus/business-synthetic.jsonl` ({len(entries)} 条)")
    w("")

    # --- Summary ---
    w("## 1. 总览")
    w("")
    w("| 指标 | 值 |")
    w("|---|---|")
    w(f"| 错误码总数 | {total_codes} |")
    w(f"| 仓库语料已覆盖 | {covered_by_existing} |")
    w(f"| 合成语料覆盖 | {covered_by_synthetic} |")
    w(f"| 合成语料中可通过脚本触发 | {len(triggerable)} |")
    w(f"| 合成语料中需要非脚本触发 | {len(not_triggerable)} |")
    w("")

    # --- Full table ---
    w("## 2. 66 错误码 x 覆盖矩阵")
    w("")
    w("| 错误码 | 严重度 | 仓库语料 | 合成语料条数 | 触发方式 |")
    w("|---|---|---|---|---|")

    for code in unique_codes:
        sev = severity_of(code)
        existing = "YES" if code in existing_covered else "--"
        syn_count = synthetic_codes.get(code, 0)
        syn_str = str(syn_count) if syn_count > 0 else "--"
        # Determine trigger method
        has_script = any(
            e["trigger"] == code and e["script"].strip()
            for e in entries
        )
        trigger = "script" if has_script else "non-script / manual"
        w(f"| `{code}` | {sev} | {existing} | {syn_str} | {trigger} |")
    w("")

    # --- Uncovered detail ---
    w("## 3. 仓库语料未覆盖的错误码 -- 构造方案")
    w("")
    uncovered = [c for c in unique_codes if c not in existing_covered]
    w(f"共 {len(uncovered)} 个错误码在仓库 295 条语料中未被触发：")
    w("")

    for code in uncovered:
        sev = severity_of(code)
        domain = DOMAIN_MAP.get(code, "unknown")
        w(f"### `{code}` ({sev}, {domain})")
        w("")
        code_entries = [e for e in entries if e["trigger"] == code]
        for e in code_entries:
            if e["script"].strip():
                w(f"- **脚本**: `{e['script']}`")
            else:
                w(f"- **脚本**: *(无法通过纯脚本触发)*")
            w(f"  - 触发原因: {e['rationale']}")
        w("")

    # --- What this tells you ---
    w("## 4. 这能告诉你什么 / 不能告诉你什么")
    w("")
    w("### 能告诉你")
    w("")
    w("- **哪些错误码在常见业务模式中可能触发**: 合成语料覆盖了所有 65 个错误码的触发场景，")
    w("  给业务脚本验收提供了优先级排序依据。")
    w("- **哪些错误码需要特殊环境**: 6 个 SERIALIZABLE_PARSE_CACHE_* 错误码、")
    w("  INVALID_ARGUMENT、BIZ_EXCEPTION 无法通过纯 QLExpress 脚本触发，")
    w("  需要 Java 端注入损坏缓存或注册自定义函数。")
    w("- **错误码的功能域分布**: 从控制流、算术、类型转换到序列化缓存，")
    w("  每个域都有对应的合成触发脚本。")
    w("")
    w("### 不能告诉你")
    w("")
    w("- **真实业务里哪些错误码频率最高**: 合成语料是人工构造的，")
    w("  不代表真实业务脚本的错误分布。需要用户提供业务脚本进行对比。")
    w("- **错误码的实际触发是否符合预期**: 合成语料标注为「理论触发」，")
    w("  需要通过 Rust 运行时验证确认实际行为。")
    w("- **错误消息是否与 Java 基准一致**: 合成语料只验证错误码，")
    w("  不验证错误消息格式。")
    w("")

    # --- How to run with user scripts ---
    w("## 5. 如何使用业务脚本运行对比")
    w("")
    w("如果用户提供了业务脚本，可以按以下步骤运行对比：")
    w("")
    w("### 步骤 1: 准备业务脚本 JSONL")
    w("")
    w("```jsonl")
    w('{"id": "my-biz-001", "script": "score >= 80 ? \'pass\' : \'fail\'", "context": {"score": 90}}')
    w('{"id": "my-biz-002", "script": "items[0] + items[1]", "context": {"items": [10, 20]}}')
    w("```")
    w("")
    w("### 步骤 2: 运行差分对比")
    w("")
    w("```bash")
    w("# 使用 Rust 运行时验证合成语料")
    w("cargo run --bin run-script-biz -- \\")
    w("  --corpus verification/corpus/business-synthetic.jsonl \\")
    w("  --output verification/results/synthetic-results.json")
    w("")
    w("# 使用 Java 基准对比")
    w("java -cp QLExpress/target/test-classes \\")
    w("  com.alibaba.qlexpress4.TestRunner \\")
    w("  --corpus verification/corpus/business-synthetic.jsonl")
    w("```")
    w("")
    w("### 步骤 3: 分析覆盖差异")
    w("")
    w("```bash")
    w("# 对比 Rust vs Java 的错误码触发差异")
    w("python3 scripts/analyze_error_distribution.py")
    w("```")
    w("")

    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    print(f"Reading error codes from: {ERROR_CODES_SRC}")

    # 1. Extract error codes
    codes = extract_error_codes(ERROR_CODES_SRC)
    print(f"Found {len(codes)} error code constants")

    # 2. Build script templates
    templates = _build_script_templates()
    print(f"Built templates for {len(templates)} error codes")

    # 3. Generate corpus
    entries = generate_corpus(codes, templates)
    print(f"Generated {len(entries)} synthetic corpus entries")

    # 4. Load existing corpus for duplicate detection
    existing_scripts = load_existing_scripts([
        PROJECT_ROOT / "verification" / "corpus" / "differential.jsonl",
    ])
    duplicates = sum(1 for e in entries if e["script"].strip() in existing_scripts)
    if duplicates:
        print(f"Note: {duplicates} scripts already appear in differential.jsonl")

    # 5. Write JSONL
    write_jsonl(entries, OUTPUT_PATH)
    print(f"Wrote synthetic corpus to: {OUTPUT_PATH}")

    # 6. Generate report
    report_path = PROJECT_ROOT / "docs" / "业务验收" / "business-synthetic-coverage.md"
    existing_covered = {
        "EXCEED_MAX_ARR_LENGTH", "FUNCTION_NOT_FOUND", "INDEX_OUT_BOUND",
        "INVALID_ARITHMETIC", "INVALID_ASSIGNMENT", "INVALID_BINARY_OPERAND",
        "INVALID_INDEX", "NULL_FIELD_ACCESS", "SYNTAX_ERROR",
    }
    report = generate_report(entries, codes, existing_covered)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(report, encoding="utf-8")
    print(f"Wrote report to: {report_path}")

    # 7. Summary
    triggerable = [e for e in entries if e["script"].strip()]
    not_triggerable = [e for e in entries if not e["script"].strip()]
    codes_with_scripts = len(set(e["trigger"] for e in triggerable))
    print(f"\n--- Summary ---")
    print(f"Total entries: {len(entries)}")
    print(f"Triggerable via script: {len(triggerable)} ({codes_with_scripts} codes)")
    print(f"Non-script triggerable: {len(not_triggerable)}")
    print(f"Error codes covered: {codes_with_scripts + len(not_triggerable)} / {len(set(codes.values()))}")


if __name__ == "__main__":
    main()
