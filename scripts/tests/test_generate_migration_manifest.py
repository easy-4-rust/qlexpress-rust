"""迁移清单人工证据校验器的回归测试。"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.generate_migration_manifest import (
    apply_dispositions,
    java_key,
    load_dispositions,
    source_tree_fingerprint,
    validate_disposition,
)


JAVA_KEY = "com.alibaba.qlexpress4::Runner::execute|Object (String script)"


class MigrationDispositionTest(unittest.TestCase):
    """验证人工 disposition 不能绕过基线和证据约束。"""

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.rust_root = Path(self.temp_dir.name)
        source = self.rust_root / "crates/qlexpress/src/runner.rs"
        source.parent.mkdir(parents=True)
        source.write_text(
            "pub fn execute(script: &str) -> DataValue { todo_body(script) }\n",
            encoding="utf-8",
        )
        test = self.rust_root / "crates/qlexpress/tests/runner.rs"
        test.parent.mkdir(parents=True)
        test.write_text(
            "#[test]\nfn execute_matches_java() { assert_eq!(1, 1); }\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def implemented(self) -> dict[str, object]:
        """构造一条具备语义、源码和测试锚点的有效记录。"""
        return {
            "java_key": JAVA_KEY,
            "state": "IMPLEMENTED",
            "classification": "ADAPTED",
            "semantic_evidence": [
                "Rust execute 保留 Java execute 的脚本输入、结果和错误契约。"
            ],
            "rust_evidence": [
                {
                    "file": "crates/qlexpress/src/runner.rs",
                    "symbol": "pub fn execute",
                }
            ],
            "test_evidence": [
                {
                    "file": "crates/qlexpress/tests/runner.rs",
                    "test": "execute_matches_java",
                    "level": "V2_MIRRORED",
                }
            ],
            "review_note": "已逐项核对参数、返回值和错误。",
        }

    def test_java_key_normalizes_signature_whitespace(self) -> None:
        java = {
            "qualified_name": "com.alibaba.qlexpress4::Runner::execute",
            "signature": "Object   (String\n script)",
        }
        self.assertEqual(java_key(java), JAVA_KEY)

    def test_implemented_requires_test_evidence(self) -> None:
        disposition = self.implemented()
        disposition["test_evidence"] = []
        with self.assertRaisesRegex(ValueError, "requires test_evidence"):
            validate_disposition(disposition, rust_root=self.rust_root)

    def test_evidence_symbol_must_exist_in_current_file(self) -> None:
        disposition = self.implemented()
        disposition["rust_evidence"][0]["symbol"] = "missing_symbol"
        with self.assertRaisesRegex(ValueError, "not found"):
            validate_disposition(disposition, rust_root=self.rust_root)

    def test_load_rejects_stale_baseline(self) -> None:
        path = self.rust_root / "dispositions.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "java_baseline": "old-java",
                    "rust_baseline": "rust-sha",
                    "rust_source_fingerprint": "sha256:stale",
                    "objects": [],
                    "nested_java_types": [],
                    "methods": [self.implemented()],
                }
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValueError, "Java baseline"):
            load_dispositions(
                path,
                java_sha="java-sha",
                rust_sha="rust-sha",
                rust_source_fingerprint=source_tree_fingerprint(
                    self.rust_root,
                    Path("crates/qlexpress/src"),
                ),
                rust_root=self.rust_root,
            )

    def test_load_rejects_stale_source_fingerprint(self) -> None:
        path = self.rust_root / "dispositions.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "java_baseline": "java-sha",
                    "rust_baseline": "rust-sha",
                    "rust_source_fingerprint": "sha256:stale",
                    "objects": [],
                    "nested_java_types": [],
                    "methods": [self.implemented()],
                }
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValueError, "source fingerprint"):
            load_dispositions(
                path,
                java_sha="java-sha",
                rust_sha="rust-sha",
                rust_source_fingerprint=source_tree_fingerprint(
                    self.rust_root,
                    Path("crates/qlexpress/src"),
                ),
                rust_root=self.rust_root,
            )

    def test_load_rejects_unrelated_rust_baseline(self) -> None:
        fingerprint = source_tree_fingerprint(
            self.rust_root,
            Path("crates/qlexpress/src"),
        )
        path = self.rust_root / "dispositions.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "java_baseline": "java-sha",
                    "rust_baseline": "unrelated-rust-sha",
                    "rust_source_fingerprint": fingerprint,
                    "objects": [],
                    "nested_java_types": [],
                    "methods": [self.implemented()],
                }
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValueError, "Rust baseline"):
            load_dispositions(
                path,
                java_sha="java-sha",
                rust_sha="current-rust-sha",
                rust_source_fingerprint=fingerprint,
                rust_root=self.rust_root,
            )

    def test_load_expands_grouped_java_keys(self) -> None:
        second_key = (
            "com.alibaba.qlexpress4::Runner::execute|"
            "Object (String script, Object context)"
        )
        grouped = self.implemented()
        grouped.pop("java_key")
        grouped["java_keys"] = [JAVA_KEY, second_key]
        fingerprint = source_tree_fingerprint(
            self.rust_root,
            Path("crates/qlexpress/src"),
        )
        path = self.rust_root / "dispositions.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "java_baseline": "java-sha",
                    "rust_baseline": "rust-sha",
                    "rust_source_fingerprint": fingerprint,
                    "objects": [],
                    "nested_java_types": [],
                    "methods": [grouped],
                }
            ),
            encoding="utf-8",
        )
        loaded = load_dispositions(
            path,
            java_sha="java-sha",
            rust_sha="rust-sha",
            rust_source_fingerprint=fingerprint,
            rust_root=self.rust_root,
        )
        self.assertEqual(set(loaded["methods"]), {JAVA_KEY, second_key})

    def test_exact_reviewed_key_promotes_row(self) -> None:
        disposition = validate_disposition(
            self.implemented(),
            rust_root=self.rust_root,
        )
        rows = [
            {
                "java": {
                    "qualified_name": "com.alibaba.qlexpress4::Runner::execute",
                    "signature": "Object (String script)",
                },
                "state": "MISSING",
                "semantic_evidence": [],
                "review_note": "未发现候选。",
            }
        ]
        stats = apply_dispositions(
            rows,
            {JAVA_KEY: disposition},
            section="methods",
        )
        self.assertEqual(rows[0]["state"], "IMPLEMENTED")
        self.assertEqual(stats, {"provided": 1, "matched": 1, "handled": 1})

    def test_unmatched_reviewed_key_is_rejected(self) -> None:
        disposition = validate_disposition(
            self.implemented(),
            rust_root=self.rust_root,
        )
        with self.assertRaisesRegex(ValueError, "unmatched methods"):
            apply_dispositions([], {JAVA_KEY: disposition}, section="methods")


if __name__ == "__main__":
    unittest.main()
