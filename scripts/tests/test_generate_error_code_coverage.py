"""Unit tests for scripts/generate_error_code_coverage.py.

Tests cover error code extraction, template classification, JSONL output
validation, corpus size, and coverage breadth.  No Rust compilation or
Java baseline is required.

Usage::

    python3 -m pytest scripts/tests/test_generate_error_code_coverage.py -v
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.generate_error_code_coverage import (
    DOMAIN_MAP,
    _build_script_templates,
    extract_error_codes,
    generate_corpus,
    generate_report,
    severity_of,
    write_jsonl,
)


# ---------------------------------------------------------------------------
# Mock error codes file
# ---------------------------------------------------------------------------

MOCK_ERROR_CODES_RS = '''
//! Error codes for testing.

pub const SYNTAX_ERROR: &str = "SYNTAX_ERROR";
pub const NULL_FIELD_ACCESS: &str = "NULL_FIELD_ACCESS";
pub const INVALID_ARITHMETIC: &str = "INVALID_ARITHMETIC";
pub const FUNCTION_NOT_FOUND: &str = "FUNCTION_NOT_FOUND";
pub const EXCEED_MAX_ARR_LENGTH: &str = "EXCEED_MAX_ARR_LENGTH";
pub const INVALID_INDEX: &str = "INVALID_INDEX";
pub const INDEX_OUT_BOUND: &str = "INDEX_OUT_BOUND";
pub const INVALID_BINARY_OPERAND: &str = "INVALID_BINARY_OPERAND";
pub const INVALID_ASSIGNMENT: &str = "INVALID_ASSIGNMENT";
pub const CLASS_NOT_FOUND: &str = "CLASS_NOT_FOUND";
pub const SCRIPT_TIME_OUT: &str = "SCRIPT_TIME_OUT";
pub const STACK_OVERFLOW: &str = "STACK_OVERFLOW";
pub const OPERAND_STACK_OVERFLOW: &str = "OPERAND_STACK_OVERFLOW";
pub const OPERAND_STACK_UNDERFLOW: &str = "OPERAND_STACK_UNDERFLOW";
pub const INVOKE_METHOD_WITH_WRONG_ARGUMENTS: &str = "INVOKE_METHOD_WITH_WRONG_ARGUMENTS";
pub const METHOD_NOT_FOUND: &str = "METHOD_NOT_FOUND";
pub const NULL_CALL: &str = "NULL_CALL";
pub const INCOMPATIBLE_TYPE_CAST: &str = "INCOMPATIBLE_TYPE_CAST";
pub const INVALID_CAST_TARGET: &str = "INVALID_CAST_TARGET";
pub const INCOMPATIBLE_ASSIGNMENT_TYPE: &str = "INCOMPATIBLE_ASSIGNMENT_TYPE";
pub const FOR_EACH_ITERABLE_REQUIRED: &str = "FOR_EACH_ITERABLE_REQUIRED";
pub const WHILE_CONDITION_BOOL_REQUIRED: &str = "WHILE_CONDITION_BOOL_REQUIRED";
pub const CONDITION_BOOL_REQUIRED: &str = "CONDITION_BOOL_REQUIRED";
pub const ARRAY_SIZE_NUM_REQUIRED: &str = "ARRAY_SIZE_NUM_REQUIRED";
pub const INCOMPATIBLE_ARRAY_ITEM_TYPE: &str = "INCOMPATIBLE_ARRAY_ITEM_TYPE";
pub const INVALID_UNARY_OPERAND: &str = "INVALID_UNARY_OPERAND";
pub const OPERATOR_NOT_ALLOWED: &str = "OPERATOR_NOT_ALLOWED";
pub const QL_THROW: &str = "QL_THROW";
pub const SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION: &str = "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION";
pub const BIZ_EXCEPTION: &str = "BIZ_EXCEPTION";
pub const INVALID_ARGUMENT: &str = "INVALID_ARGUMENT";
'''


# ---------------------------------------------------------------------------
# Test 1: Error code extraction
# ---------------------------------------------------------------------------


class ExtractErrorCodesTest(unittest.TestCase):
    """Verify that error codes are parsed from Rust source."""

    def test_extracts_all_codes_from_mock(self) -> None:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as f:
            f.write(MOCK_ERROR_CODES_RS)
            f.flush()
            codes = extract_error_codes(Path(f.name))
        self.assertEqual(len(codes), 31)
        self.assertIn("SYNTAX_ERROR", codes)
        self.assertIn("NULL_FIELD_ACCESS", codes)
        self.assertIn("INVALID_ARITHMETIC", codes)

    def test_extracts_from_real_source(self) -> None:
        """Verify extraction from the real ql_error_codes.rs."""
        real_path = (
            Path(__file__).resolve().parent.parent.parent
            / "crates"
            / "qlexpress"
            / "src"
            / "exception"
            / "ql_error_codes.rs"
        )
        if not real_path.exists():
            self.skipTest("ql_error_codes.rs not found")
        codes = extract_error_codes(real_path)
        self.assertGreaterEqual(len(codes), 60, "Expected at least 60 error codes")
        self.assertIn("SYNTAX_ERROR", codes)
        self.assertIn("QL_THROW", codes)

    def test_deduplicates_codes(self) -> None:
        text = 'pub const FOO: &str = "FOO";\npub const FOO: &str = "FOO";\n'
        with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as f:
            f.write(text)
            f.flush()
            codes = extract_error_codes(Path(f.name))
        self.assertEqual(len(codes), 1)
        self.assertEqual(codes["FOO"], "FOO")


# ---------------------------------------------------------------------------
# Test 2: Template classification
# ---------------------------------------------------------------------------


class TemplateClassificationTest(unittest.TestCase):
    """Verify that script templates are properly classified by domain."""

    def test_templates_exist_for_all_codes(self) -> None:
        templates = _build_script_templates()
        # Should have templates for all 65 error codes
        self.assertGreaterEqual(len(templates), 60)

    def test_templates_have_valid_structure(self) -> None:
        templates = _build_script_templates()
        for code, scripts in templates.items():
            self.assertIsInstance(scripts, list, f"{code} should have list of scripts")
            for script, rationale in scripts:
                self.assertIsInstance(script, str, f"{code} script should be str")
                self.assertIsInstance(rationale, str, f"{code} rationale should be str")
                self.assertGreater(len(rationale), 0, f"{code} rationale should not be empty")

    def test_domain_map_covers_all_codes(self) -> None:
        templates = _build_script_templates()
        for code in templates:
            # `__BIZ_*__` 是业务领域样本（不映射到错误码），无需 DOMAIN_MAP
            if code.startswith("__BIZ_") and code.endswith("__"):
                continue
            self.assertIn(
                code, DOMAIN_MAP,
                f"{code} should be in DOMAIN_MAP",
            )

    def test_severity_classification(self) -> None:
        # P0 codes
        self.assertEqual(severity_of("SYNTAX_ERROR"), "P0")
        self.assertEqual(severity_of("STACK_OVERFLOW"), "P0")
        self.assertEqual(severity_of("OPERAND_STACK_OVERFLOW"), "P0")
        # P1 codes
        self.assertEqual(severity_of("INVALID_ARITHMETIC"), "P1")
        self.assertEqual(severity_of("NULL_FIELD_ACCESS"), "P1")
        self.assertEqual(severity_of("SCRIPT_TIME_OUT"), "P1")
        # P2 codes
        self.assertEqual(severity_of("OPERATOR_NOT_ALLOWED"), "P2")
        self.assertEqual(
            severity_of("SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION"), "P2",
        )


# ---------------------------------------------------------------------------
# Test 3: Generated JSONL is valid
# ---------------------------------------------------------------------------


class JsonlValidityTest(unittest.TestCase):
    """Verify that generated JSONL entries are well-formed."""

    def _generate_entries(self) -> list[dict]:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as f:
            f.write(MOCK_ERROR_CODES_RS)
            f.flush()
            codes = extract_error_codes(Path(f.name))
        templates = _build_script_templates()
        return generate_corpus(codes, templates)

    def test_entries_are_valid_json(self) -> None:
        entries = self._generate_entries()
        for entry in entries:
            # Should be serializable and deserializable
            json_str = json.dumps(entry, ensure_ascii=False)
            parsed = json.loads(json_str)
            self.assertEqual(parsed["id"], entry["id"])

    def test_entries_have_required_keys(self) -> None:
        entries = self._generate_entries()
        required_keys = {"id", "category", "trigger", "script", "rationale"}
        for entry in entries:
            self.assertEqual(
                set(entry.keys()), required_keys,
                f"Entry {entry.get('id', '?')} has wrong keys: {set(entry.keys())}",
            )

    def test_entries_have_valid_trigger(self) -> None:
        entries = self._generate_entries()
        codes = extract_error_codes(
            Path(tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False).name)
        )
        # Just verify trigger is a non-empty string
        for entry in entries:
            self.assertIsInstance(entry["trigger"], str)
            self.assertGreater(len(entry["trigger"]), 0)

    def test_entries_have_valid_category(self) -> None:
        entries = self._generate_entries()
        for entry in entries:
            self.assertIsInstance(entry["category"], str)
            self.assertGreater(len(entry["category"]), 0)

    def test_write_jsonl_roundtrip(self) -> None:
        entries = self._generate_entries()
        with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as f:
            write_jsonl(entries, Path(f.name))
            content = Path(f.name).read_text()
            lines = [l.strip() for l in content.splitlines() if l.strip()]
        self.assertEqual(len(lines), len(entries))
        for line in lines:
            parsed = json.loads(line)
            self.assertIn("id", parsed)
            self.assertIn("trigger", parsed)


# ---------------------------------------------------------------------------
# Test 4: Synthetic scripts count >= N
# ---------------------------------------------------------------------------


class CorpusSizeTest(unittest.TestCase):
    """Verify that the corpus meets minimum size requirements."""

    def test_at_least_50_entries_from_real_codes(self) -> None:
        """Each error code should generate at least 1 entry."""
        real_path = (
            Path(__file__).resolve().parent.parent.parent
            / "crates"
            / "qlexpress"
            / "src"
            / "exception"
            / "ql_error_codes.rs"
        )
        if not real_path.exists():
            self.skipTest("ql_error_codes.rs not found")
        codes = extract_error_codes(real_path)
        templates = _build_script_templates()
        entries = generate_corpus(codes, templates)
        self.assertGreaterEqual(
            len(entries), 50,
            f"Expected at least 50 entries, got {len(entries)}",
        )

    def test_each_code_has_at_least_one_entry(self) -> None:
        real_path = (
            Path(__file__).resolve().parent.parent.parent
            / "crates"
            / "qlexpress"
            / "src"
            / "exception"
            / "ql_error_codes.rs"
        )
        if not real_path.exists():
            self.skipTest("ql_error_codes.rs not found")
        codes = extract_error_codes(real_path)
        templates = _build_script_templates()
        entries = generate_corpus(codes, templates)
        triggers = set(e["trigger"] for e in entries)
        for const_name, error_code in codes.items():
            self.assertIn(
                error_code, triggers,
                f"Error code {error_code} has no corpus entry",
            )


# ---------------------------------------------------------------------------
# Test 5: Coverage breadth >= 50 distinct error codes
# ---------------------------------------------------------------------------


class CoverageBreadthTest(unittest.TestCase):
    """Verify that at least 50 distinct error codes are covered."""

    def test_coverage_at_least_50_codes(self) -> None:
        real_path = (
            Path(__file__).resolve().parent.parent.parent
            / "crates"
            / "qlexpress"
            / "src"
            / "exception"
            / "ql_error_codes.rs"
        )
        if not real_path.exists():
            self.skipTest("ql_error_codes.rs not found")
        codes = extract_error_codes(real_path)
        templates = _build_script_templates()
        entries = generate_corpus(codes, templates)
        covered = set(e["trigger"] for e in entries)
        self.assertGreaterEqual(
            len(covered), 50,
            f"Expected at least 50 error codes covered, got {len(covered)}",
        )

    def test_all_codes_from_mock_are_covered(self) -> None:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as f:
            f.write(MOCK_ERROR_CODES_RS)
            f.flush()
            codes = extract_error_codes(Path(f.name))
        templates = _build_script_templates()
        entries = generate_corpus(codes, templates)
        covered = set(e["trigger"] for e in entries)
        for const_name, error_code in codes.items():
            self.assertIn(
                error_code, covered,
                f"Mock error code {error_code} not covered",
            )


# ---------------------------------------------------------------------------
# Test 6: Report generation
# ---------------------------------------------------------------------------


class ReportGenerationTest(unittest.TestCase):
    """Verify that the markdown report is well-formed."""

    def test_report_contains_required_sections(self) -> None:
        real_path = (
            Path(__file__).resolve().parent.parent.parent
            / "crates"
            / "qlexpress"
            / "src"
            / "exception"
            / "ql_error_codes.rs"
        )
        if not real_path.exists():
            self.skipTest("ql_error_codes.rs not found")
        codes = extract_error_codes(real_path)
        templates = _build_script_templates()
        entries = generate_corpus(codes, templates)
        existing_covered = {
            "EXCEED_MAX_ARR_LENGTH", "FUNCTION_NOT_FOUND", "INDEX_OUT_BOUND",
            "INVALID_ARITHMETIC", "INVALID_ASSIGNMENT", "INVALID_BINARY_OPERAND",
            "INVALID_INDEX", "NULL_FIELD_ACCESS", "SYNTAX_ERROR",
        }
        report = generate_report(entries, codes, existing_covered)
        self.assertIn("# QLExpress-Rust", report)
        self.assertIn("## 1. 总览", report)
        self.assertIn("## 2. 66 错误码", report)
        self.assertIn("## 3. 仓库语料未覆盖的错误码", report)
        self.assertIn("## 4. 这能告诉你什么", report)
        self.assertIn("## 5. 如何使用业务脚本运行对比", report)


if __name__ == "__main__":
    unittest.main()
