"""Unit tests for scripts/analyze_error_distribution.py.

Tests cover corpus parsing, case classification, error code extraction,
and output format validation using mock data.  No Java baseline or Rust
compilation is required.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path

from scripts.analyze_error_distribution import (
    AnalysisResult,
    classify_case,
    extract_error_codes,
    severity_of,
    write_json,
    write_markdown,
)


# ---------------------------------------------------------------------------
# Mock corpus entries (minimal representative set)
# ---------------------------------------------------------------------------

MOCK_CORPUS = [
    # 1. Script case -- success
    {"id": "int-precedence", "script": "1 + 2 * 3"},
    # 2. Script case -- error (divide by zero)
    {"id": "divide-by-zero-error", "script": "1 / 0"},
    # 3. Script case -- error (null field access)
    {"id": "null-field-error", "script": "missing.child"},
    # 4. NumberMath case -- success
    {"id": "numbermath-add-int", "number_math": {
        "operation": "add",
        "left": {"type": "int", "value": "1"},
        "right": {"type": "int", "value": "2"},
    }},
    # 5. NumberMath case -- error (divide zero)
    {"id": "numbermath-error-integer-divide-zero", "number_math": {
        "operation": "divide",
        "left": {"type": "int", "value": "1"},
        "right": {"type": "int", "value": "0"},
    }},
]

# A mock error codes file with a subset of real codes.
MOCK_ERROR_CODES_RS = '''
pub const SYNTAX_ERROR: &str = "SYNTAX_ERROR";
pub const INVALID_ARITHMETIC: &str = "INVALID_ARITHMETIC";
pub const NULL_FIELD_ACCESS: &str = "NULL_FIELD_ACCESS";
pub const FUNCTION_NOT_FOUND: &str = "FUNCTION_NOT_FOUND";
pub const EXCEED_MAX_ARR_LENGTH: &str = "EXCEED_MAX_ARR_LENGTH";
pub const INVALID_INDEX: &str = "INVALID_INDEX";
pub const INDEX_OUT_BOUND: &str = "INDEX_OUT_BOUND";
pub const INVALID_BINARY_OPERAND: &str = "INVALID_BINARY_OPERAND";
pub const INVALID_ASSIGNMENT: &str = "INVALID_ASSIGNMENT";
'''


# ---------------------------------------------------------------------------
# Tests: extract_error_codes
# ---------------------------------------------------------------------------


class ExtractErrorCodesTest(unittest.TestCase):
    """Verify that error codes are parsed from Rust source."""

    def test_extracts_all_codes_from_mock(self) -> None:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as f:
            f.write(MOCK_ERROR_CODES_RS)
            f.flush()
            codes = extract_error_codes(Path(f.name))
        self.assertEqual(len(codes), 9)
        self.assertIn("SYNTAX_ERROR", codes)
        self.assertIn("INVALID_ARITHMETIC", codes)
        self.assertIn("NULL_FIELD_ACCESS", codes)

    def test_deduplicates_codes(self) -> None:
        text = 'pub const FOO: &str = "FOO";\npub const FOO: &str = "FOO";\n'
        with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as f:
            f.write(text)
            f.flush()
            codes = extract_error_codes(Path(f.name))
        self.assertEqual(codes, ["FOO"])

    def test_empty_file(self) -> None:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as f:
            f.write("")
            f.flush()
            codes = extract_error_codes(Path(f.name))
        self.assertEqual(codes, [])


# ---------------------------------------------------------------------------
# Tests: classify_case
# ---------------------------------------------------------------------------


class ClassifyCaseTest(unittest.TestCase):
    """Verify case classification logic."""

    def test_script_success(self) -> None:
        case = {"id": "int-precedence", "script": "1 + 2 * 3"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertIsNone(error_code)

    def test_script_divide_by_zero(self) -> None:
        case = {"id": "divide-by-zero-error", "script": "1 / 0"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "INVALID_ARITHMETIC")

    def test_script_null_field(self) -> None:
        case = {"id": "null-field-error", "script": "missing.child"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "NULL_FIELD_ACCESS")

    def test_script_syntax_error(self) -> None:
        case = {"id": "syntax-error", "script": "a = (1 +"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "SYNTAX_ERROR")

    def test_script_missing_function(self) -> None:
        case = {"id": "missing-function-error", "script": "noSuchFunc()"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "FUNCTION_NOT_FOUND")

    def test_script_array_limit(self) -> None:
        case = {"id": "array-limit-error", "script": "new int[11]", "options": {"max_arr_length": 10}}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "EXCEED_MAX_ARR_LENGTH")

    def test_script_invalid_index(self) -> None:
        case = {"id": "invalid-index-error", "script": "a=[1]; a['x']"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "INVALID_INDEX")

    def test_script_index_out_of_bound(self) -> None:
        case = {"id": "index-out-of-bound-error", "script": "a=[]; a[1]"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "INDEX_OUT_BOUND")

    def test_script_null_value(self) -> None:
        case = {"id": "null-value", "script": "null"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "NULL_FIELD_ACCESS")

    def test_script_invalid_operand(self) -> None:
        case = {"id": "basebinary-invalid-operand-reason", "script": "true + []"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "INVALID_BINARY_OPERAND")

    def test_script_invalid_assignment(self) -> None:
        case = {"id": "basebinary-invalid-left-value", "script": "function value(){1;} value() += 2"}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "script")
        self.assertEqual(error_code, "INVALID_ASSIGNMENT")

    def test_number_math_success(self) -> None:
        case = {
            "id": "numbermath-add-int",
            "number_math": {
                "operation": "add",
                "left": {"type": "int", "value": "1"},
                "right": {"type": "int", "value": "2"},
            },
        }
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "number_math")
        self.assertIsNone(error_code)

    def test_number_math_divide_zero(self) -> None:
        case = {
            "id": "numbermath-error-integer-divide-zero",
            "number_math": {
                "operation": "divide",
                "left": {"type": "int", "value": "1"},
                "right": {"type": "int", "value": "0"},
            },
        }
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "number_math")
        self.assertEqual(error_code, "INVALID_ARITHMETIC")

    def test_operator_manager_success(self) -> None:
        case = {
            "id": "operator-manager-get-binary-add",
            "operator_manager": {
                "operation": "getBinaryOperator",
                "lexeme": "+",
            },
        }
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "operator_manager")
        self.assertIsNone(error_code)

    def test_full_contract_sub_case(self) -> None:
        case = {"id": "exist-stack-full-contract", "exist_stack": {"scenario": "full_contract"}}
        case_type, error_code = classify_case(case)
        self.assertEqual(case_type, "exist_stack")
        self.assertIsNone(error_code)


# ---------------------------------------------------------------------------
# Tests: severity_of
# ---------------------------------------------------------------------------


class SeverityOfTest(unittest.TestCase):

    def test_p0_codes(self) -> None:
        self.assertEqual(severity_of("SYNTAX_ERROR"), "P0")
        self.assertEqual(severity_of("OPERAND_STACK_OVERFLOW"), "P0")

    def test_p1_codes(self) -> None:
        self.assertEqual(severity_of("INVALID_ARITHMETIC"), "P1")
        self.assertEqual(severity_of("NULL_FIELD_ACCESS"), "P1")

    def test_p2_codes(self) -> None:
        self.assertEqual(severity_of("OPERATOR_NOT_ALLOWED"), "P2")
        self.assertEqual(severity_of("SERIALIZABLE_PARSE_CACHE_INVALID_MODEL"), "P2")


# ---------------------------------------------------------------------------
# Tests: JSON output
# ---------------------------------------------------------------------------


class JsonOutputTest(unittest.TestCase):
    """Verify JSON output structure."""

    def test_json_has_required_keys(self) -> None:
        result = AnalysisResult(
            generated_at="2026-01-01T00:00:00+00:00",
            total_cases=5,
            success_count=3,
            error_count=2,
            all_codes=["A", "B"],
            covered_codes=["A"],
            uncovered_codes=["B"],
            by_error_code=Counter({"A": 2, "B": 0}),
            by_kind=Counter({"P1": 2, "Success": 3}),
            error_details=[
                {"id": "x", "case_type": "script", "error_code": "A", "severity": "P1"},
            ],
        )
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            write_json(result, Path(f.name))
            data = json.loads(Path(f.name).read_text())

        self.assertEqual(data["total_cases"], 5)
        self.assertEqual(data["success_count"], 3)
        self.assertEqual(data["exception_count"], 2)
        self.assertIn("by_error_code", data)
        self.assertIn("by_kind", data)
        self.assertIn("covered_codes", data)
        self.assertIn("uncovered_codes", data)
        self.assertIn("error_details", data)
        self.assertEqual(data["by_error_code"]["A"], 2)


# ---------------------------------------------------------------------------
# Tests: Markdown output
# ---------------------------------------------------------------------------


class MarkdownOutputTest(unittest.TestCase):
    """Verify Markdown report structure."""

    def test_markdown_has_sections(self) -> None:
        result = AnalysisResult(
            generated_at="2026-01-01T00:00:00+00:00",
            total_cases=5,
            success_count=3,
            error_count=2,
            all_codes=["A", "B"],
            covered_codes=["A"],
            uncovered_codes=["B"],
            by_error_code=Counter({"A": 2, "B": 0}),
            by_kind=Counter({"P1": 2, "Success": 3}),
            error_details=[
                {"id": "x", "case_type": "script", "error_code": "A", "severity": "P1"},
            ],
        )
        with tempfile.NamedTemporaryFile(suffix=".md", delete=False) as f:
            write_markdown(result, Path(f.name))
            text = Path(f.name).read_text()

        self.assertIn("# QLExpress-Rust", text)
        self.assertIn("## 1. 总览", text)
        self.assertIn("## 4. 错误码触发明细", text)
        self.assertIn("## 5. 未触发的错误码", text)
        self.assertIn("## 8. 对业务验收的启示", text)
        self.assertIn("`A`", text)


# ---------------------------------------------------------------------------
# Tests: integration with real corpus (smoke)
# ---------------------------------------------------------------------------


class RealCorpusSmokeTest(unittest.TestCase):
    """Smoke test against the real 295-entry corpus."""

    def test_total_cases_is_295(self) -> None:
        corpus_path = Path(__file__).resolve().parent.parent.parent / "verification" / "corpus" / "differential.jsonl"
        if not corpus_path.exists():
            self.skipTest("corpus not found")
        count = 0
        with open(corpus_path) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#"):
                    count += 1
        self.assertEqual(count, 295)

    def test_classify_all_cases_no_unknown(self) -> None:
        corpus_path = Path(__file__).resolve().parent.parent.parent / "verification" / "corpus" / "differential.jsonl"
        if not corpus_path.exists():
            self.skipTest("corpus not found")
        with open(corpus_path) as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                case = json.loads(line)
                case_type, _ = classify_case(case)
                self.assertNotEqual(case_type, "unknown", f"unknown type for case {case['id']}")


if __name__ == "__main__":
    unittest.main()
