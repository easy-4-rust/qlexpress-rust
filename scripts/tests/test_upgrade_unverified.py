"""Unit tests for scripts/upgrade_unverified.py.

Tests cover argument parsing, pure functions, and path generation.
End-to-end workflow tests require a Java mirror repository and are
not included here.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.upgrade_unverified import (
    UpgradeConfig,
    count_states,
    format_owner_distribution,
    format_state_table,
    parse_args,
    read_cargo_baseline_sha,
    source_tree_fingerprint,
    validate_sha,
    worktree_path_for_sha,
)


VALID_SHA = "9065b9ac5d985dcd02e627239aa9cdb78fb2f7f3"
SHORT_SHA = "9065b9ac"


class ParseArgsTest(unittest.TestCase):
    """Test command-line argument parsing for all subcommands."""

    def test_check_minimal(self) -> None:
        args = parse_args([
            "check",
            "--java-repo", "/tmp/java",
            "--java-baseline-sha", VALID_SHA,
        ])
        self.assertEqual(args.command, "check")
        self.assertEqual(args.java_repo, Path("/tmp/java"))
        self.assertEqual(args.java_baseline_sha, VALID_SHA)
        self.assertEqual(args.rust_root, Path("."))

    def test_check_with_all_options(self) -> None:
        args = parse_args([
            "check",
            "--java-repo", "/tmp/java",
            "--rust-root", "/tmp/rust",
            "--java-baseline-sha", VALID_SHA,
            "--worktree-root", "/tmp/worktrees",
            "--rust-source-root", "crates/custom/src",
            "--java-package-root-suffix", "src/main/java/com/example",
        ])
        self.assertEqual(args.command, "check")
        self.assertEqual(args.rust_root, Path("/tmp/rust"))
        self.assertEqual(args.worktree_root, Path("/tmp/worktrees"))
        self.assertEqual(args.rust_source_root, Path("crates/custom/src"))
        self.assertEqual(
            args.java_package_root_suffix,
            "src/main/java/com/example",
        )

    def test_apply_minimal(self) -> None:
        args = parse_args([
            "apply",
            "--java-repo", "/tmp/java",
            "--java-baseline-sha", VALID_SHA,
        ])
        self.assertEqual(args.command, "apply")
        self.assertFalse(args.skip_codegraph)
        self.assertFalse(args.skip_test_inventory)
        self.assertIsNone(args.test_inventory_cmd)

    def test_apply_with_skip_flags(self) -> None:
        args = parse_args([
            "apply",
            "--java-repo", "/tmp/java",
            "--java-baseline-sha", VALID_SHA,
            "--skip-codegraph",
            "--skip-test-inventory",
            "--test-inventory-cmd", "echo hello",
        ])
        self.assertTrue(args.skip_codegraph)
        self.assertTrue(args.skip_test_inventory)
        self.assertEqual(args.test_inventory_cmd, "echo hello")

    def test_clean_minimal(self) -> None:
        args = parse_args([
            "clean",
            "--java-repo", "/tmp/java",
            "--java-baseline-sha", VALID_SHA,
        ])
        self.assertEqual(args.command, "clean")

    def test_missing_required_args(self) -> None:
        with self.assertRaises(SystemExit):
            parse_args(["check", "--java-repo", "/tmp/java"])

    def test_missing_command(self) -> None:
        with self.assertRaises(SystemExit):
            parse_args(["--java-repo", "/tmp/java", "--java-baseline-sha", VALID_SHA])


class ValidateShaTest(unittest.TestCase):
    """Test SHA validation."""

    def test_valid_sha(self) -> None:
        self.assertTrue(validate_sha(VALID_SHA))

    def test_short_sha_rejected(self) -> None:
        self.assertFalse(validate_sha("9065b9ac"))

    def test_long_sha_rejected(self) -> None:
        self.assertFalse(validate_sha(VALID_SHA + "ab"))

    def test_non_hex_rejected(self) -> None:
        self.assertFalse(validate_sha("g" * 40))

    def test_empty_rejected(self) -> None:
        self.assertFalse(validate_sha(""))

    def test_uppercase_hex_accepted(self) -> None:
        self.assertTrue(validate_sha("A" * 40))


class WorktreePathTest(unittest.TestCase):
    """Test worktree path generation."""

    def test_deterministic(self) -> None:
        root = Path("/tmp").resolve()
        path = worktree_path_for_sha(VALID_SHA, root)
        expected = root / f"{WORKTREE_PREFIX}{SHORT_SHA}"
        self.assertEqual(path, expected)
        # Run twice — same result
        path2 = worktree_path_for_sha(VALID_SHA, root)
        self.assertEqual(path, path2)

    def test_different_shas_different_paths(self) -> None:
        other_sha = "abcdef1234567890abcdef1234567890abcdef12"
        root = Path("/tmp").resolve()
        path1 = worktree_path_for_sha(VALID_SHA, root)
        path2 = worktree_path_for_sha(other_sha, root)
        self.assertNotEqual(path1, path2)

    def test_custom_root(self) -> None:
        path = worktree_path_for_sha(VALID_SHA, Path("/data/worktrees"))
        expected = Path("/data/worktrees").resolve() / f"{WORKTREE_PREFIX}{SHORT_SHA}"
        self.assertEqual(path, expected)


WORKTREE_PREFIX = "qlx-baseline-"


class SourceTreeFingerprintTest(unittest.TestCase):
    """Test source tree fingerprint computation."""

    def test_empty_tree(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "src"
            source.mkdir()
            fp = source_tree_fingerprint(root, Path("src"))
            self.assertTrue(fp.startswith("sha256:"))
            # Should be deterministic
            fp2 = source_tree_fingerprint(root, Path("src"))
            self.assertEqual(fp, fp2)

    def test_changes_invalidate_fingerprint(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "src"
            source.mkdir()
            (source / "lib.rs").write_text("fn main() {}", encoding="utf-8")
            fp1 = source_tree_fingerprint(root, Path("src"))
            (source / "lib.rs").write_text("fn main() { println!(\"hi\"); }", encoding="utf-8")
            fp2 = source_tree_fingerprint(root, Path("src"))
            self.assertNotEqual(fp1, fp2)

    def test_new_file_invalidates_fingerprint(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "src"
            source.mkdir()
            (source / "lib.rs").write_text("fn main() {}", encoding="utf-8")
            fp1 = source_tree_fingerprint(root, Path("src"))
            (source / "extra.rs").write_text("// new", encoding="utf-8")
            fp2 = source_tree_fingerprint(root, Path("src"))
            self.assertNotEqual(fp1, fp2)

    def test_path_ordering_deterministic(self) -> None:
        """Fingerprint must be independent of filesystem iteration order."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "src"
            source.mkdir()
            # Create files in one order
            for name in ["a.rs", "b.rs", "c.rs"]:
                (source / name).write_text(f"// {name}", encoding="utf-8")
            fp1 = source_tree_fingerprint(root, Path("src"))
            # Recreate in reverse order
            for name in ["c.rs", "b.rs", "a.rs"]:
                (source / name).write_text(f"// {name}", encoding="utf-8")
            fp2 = source_tree_fingerprint(root, Path("src"))
            self.assertEqual(fp1, fp2)

    def test_nested_files_included(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "src"
            (source / "sub").mkdir(parents=True)
            (source / "lib.rs").write_text("// root", encoding="utf-8")
            fp1 = source_tree_fingerprint(root, Path("src"))
            (source / "sub" / "mod.rs").write_text("// sub", encoding="utf-8")
            fp2 = source_tree_fingerprint(root, Path("src"))
            self.assertNotEqual(fp1, fp2)


class ReadCargoBaselineShaTest(unittest.TestCase):
    """Test reading java-baseline-commit from Cargo.toml."""

    def test_reads_sha(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cargo = Path(tmp) / "Cargo.toml"
            cargo.write_text(
                '[workspace.metadata.qlexpress]\n'
                'java-baseline-version = "4.2.0-beta"\n'
                f'java-baseline-commit = "{VALID_SHA}"\n',
                encoding="utf-8",
            )
            result = read_cargo_baseline_sha(cargo)
            self.assertEqual(result, VALID_SHA)

    def test_missing_file(self) -> None:
        result = read_cargo_baseline_sha(Path("/nonexistent/Cargo.toml"))
        self.assertIsNone(result)

    def test_missing_key(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cargo = Path(tmp) / "Cargo.toml"
            cargo.write_text(
                '[workspace.dependencies]\nserde = "1"\n',
                encoding="utf-8",
            )
            result = read_cargo_baseline_sha(cargo)
            self.assertIsNone(result)

    def test_single_quoted_value(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cargo = Path(tmp) / "Cargo.toml"
            cargo.write_text(
                f"[workspace.metadata.qlexpress]\n"
                f"java-baseline-commit = '{VALID_SHA}'\n",
                encoding="utf-8",
            )
            result = read_cargo_baseline_sha(cargo)
            self.assertEqual(result, VALID_SHA)


class CountStatesTest(unittest.TestCase):
    """Test state counting."""

    def test_empty(self) -> None:
        self.assertEqual(count_states([]), {})

    def test_single_state(self) -> None:
        rows = [{"state": "IMPLEMENTED"}, {"state": "IMPLEMENTED"}]
        self.assertEqual(count_states(rows), {"IMPLEMENTED": 2})

    def test_multiple_states(self) -> None:
        rows = [
            {"state": "IMPLEMENTED"},
            {"state": "UNVERIFIED"},
            {"state": "MISSING"},
            {"state": "IMPLEMENTED"},
        ]
        result = count_states(rows)
        self.assertEqual(result["IMPLEMENTED"], 2)
        self.assertEqual(result["UNVERIFIED"], 1)
        self.assertEqual(result["MISSING"], 1)

    def test_missing_state_key(self) -> None:
        rows = [{"state": "IMPLEMENTED"}, {}]
        result = count_states(rows)
        self.assertEqual(result["IMPLEMENTED"], 1)
        self.assertEqual(result["UNKNOWN"], 1)


class FormatStateTableTest(unittest.TestCase):
    """Test state table formatting."""

    def test_nonempty(self) -> None:
        table = format_state_table("Methods", {"IMPLEMENTED": 100, "UNVERIFIED": 5})
        self.assertIn("Methods", table)
        self.assertIn("IMPLEMENTED", table)
        self.assertIn("UNVERIFIED", table)
        self.assertIn("100", table)
        self.assertIn("5", table)

    def test_empty(self) -> None:
        table = format_state_table("Empty", {})
        self.assertIn("Empty", table)
        self.assertIn("0 total", table)


class FormatOwnerDistributionTest(unittest.TestCase):
    """Test owner distribution formatting."""

    def test_with_java_owner(self) -> None:
        rows = [
            {"java_owner": {"qualified_name": "com.example.Foo"}, "state": "MISSING"},
            {"java_owner": {"qualified_name": "com.example.Foo"}, "state": "UNVERIFIED"},
            {"java_owner": {"qualified_name": "com.example.Bar"}, "state": "MISSING"},
        ]
        result = format_owner_distribution(rows)
        self.assertIn("Foo", result)
        self.assertIn("Bar", result)

    def test_without_java_owner(self) -> None:
        rows = [
            {"java": {"qualified_name": "com.example::Runner::execute"}, "state": "MISSING"},
        ]
        result = format_owner_distribution(rows)
        self.assertIn("Runner", result)


class UpgradeConfigTest(unittest.TestCase):
    """Test UpgradeConfig derived paths."""

    def test_derived_paths(self) -> None:
        cfg = UpgradeConfig(
            java_repo=Path("/tmp/java"),
            rust_root=Path("/tmp/rust"),
            java_baseline_sha=VALID_SHA,
        )
        self.assertEqual(cfg.short_sha, SHORT_SHA)
        expected_worktree = Path("/tmp").resolve() / f"{WORKTREE_PREFIX}{SHORT_SHA}"
        self.assertEqual(cfg.worktree_path, expected_worktree)
        self.assertEqual(cfg.java_package_root, cfg.worktree_path / "src/main/java/com/alibaba/qlexpress4")

    def test_custom_worktree_root(self) -> None:
        cfg = UpgradeConfig(
            java_repo=Path("/tmp/java"),
            rust_root=Path("/tmp/rust"),
            java_baseline_sha=VALID_SHA,
            worktree_root=Path("/data/wt"),
        )
        expected = Path("/data/wt").resolve() / f"{WORKTREE_PREFIX}{SHORT_SHA}"
        self.assertEqual(cfg.worktree_path, expected)


if __name__ == "__main__":
    unittest.main()
