#!/usr/bin/env python3
"""Automate the UNVERIFIED upgrade workflow for qlexpress-rust.

When upstream QLExpress4 publishes a new release, migration dispositions
become stale and new UNVERIFIED rows appear.  This script turns the manual
upgrade steps into a single reproducible command.

Usage::

    python3 scripts/upgrade_unverified.py check \\
        --java-repo /path/to/QLExpress \\
        --java-baseline-sha <40-hex-sha>

    python3 scripts/upgrade_unverified.py apply \\
        --java-repo /path/to/QLExpress \\
        --java-baseline-sha <40-hex-sha>

    python3 scripts/upgrade_unverified.py clean \\
        --java-baseline-sha <40-hex-sha>

Design principles:

- **Idempotent**: ``apply`` produces identical output when sources have not
  changed.  An existing worktree at the correct SHA is reused, not rebuilt.
- **Recoverable**: ``apply`` computes and validates the source fingerprint
  *before* writing committed artifacts.  A mid-step failure leaves the
  committed manifest pair untouched.
- **Auditable**: every step prints its command, elapsed time, and result
  summary to stdout.
- **Stdlib-only**: no third-party Python packages required.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SHORT_SHA_LEN = 8
WORKTREE_PREFIX = "qlx-baseline-"
DEFAULT_WORKTREE_ROOT = Path("/tmp")
DEFAULT_RUST_SOURCE_ROOT = Path("crates/qlexpress/src")
MANIFEST_CURRENT = "verification/migration-manifest-current.json"
MANIFEST_CURRENT_SUMMARY = "verification/migration-manifest-current-summary.json"
DISPOSITIONS_FILE = "verification/migration-dispositions.json"
CARGO_TOML = "Cargo.toml"


# ---------------------------------------------------------------------------
# Timed step runner
# ---------------------------------------------------------------------------


@dataclass
class StepResult:
    """Outcome of a single upgrade step."""

    name: str
    success: bool
    elapsed_seconds: float
    summary: str = ""
    error: str = ""


@dataclass
class UpgradeConfig:
    """Resolved paths and parameters for one upgrade run."""

    java_repo: Path
    rust_root: Path
    java_baseline_sha: str
    worktree_root: Path = DEFAULT_WORKTREE_ROOT
    rust_source_root: Path = DEFAULT_RUST_SOURCE_ROOT
    java_package_root_suffix: str = "src/main/java/com/alibaba/qlexpress4"
    test_inventory_cmd: str | None = None

    # Derived paths
    worktree_path: Path = field(init=False)
    short_sha: str = field(init=False)
    java_db: Path = field(init=False)
    rust_db: Path = field(init=False)

    def __post_init__(self) -> None:
        self.java_repo = self.java_repo.resolve()
        self.rust_root = self.rust_root.resolve()
        self.worktree_root = self.worktree_root.resolve()
        self.short_sha = self.java_baseline_sha[:SHORT_SHA_LEN]
        self.worktree_path = self.worktree_root / f"{WORKTREE_PREFIX}{self.short_sha}"
        self.java_db = self.worktree_path / ".codegraph" / "codegraph.db"
        self.rust_db = self.rust_root / ".codegraph" / "codegraph.db"

    @property
    def java_package_root(self) -> Path:
        return self.worktree_path / self.java_package_root_suffix

    @property
    def dispositions_path(self) -> Path:
        return self.rust_root / DISPOSITIONS_FILE

    @property
    def manifest_current_path(self) -> Path:
        return self.rust_root / MANIFEST_CURRENT

    @property
    def manifest_summary_path(self) -> Path:
        return self.rust_root / MANIFEST_CURRENT_SUMMARY


def run_step(
    name: str,
    command: list[str],
    *,
    cwd: Path | None = None,
    capture: bool = True,
    check: bool = True,
    env: dict[str, str] | None = None,
) -> StepResult:
    """Execute a subprocess, printing command and timing."""
    display_cmd = " ".join(command)
    print(f"\n{'='*60}")
    print(f"STEP: {name}")
    print(f"  CMD: {display_cmd}")
    if cwd:
        print(f"  CWD: {cwd}")
    print(f"{'='*60}")

    start = time.monotonic()
    merged_env = {**os.environ}
    if env:
        merged_env.update(env)

    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            capture_output=capture,
            text=True,
            env=merged_env,
        )
        elapsed = time.monotonic() - start
        if check and result.returncode != 0:
            stderr_tail = (result.stderr or "")[-500:]
            print(f"  FAIL (exit {result.returncode}) in {elapsed:.1f}s")
            if stderr_tail:
                print(f"  STDERR (tail):\n{stderr_tail}")
            return StepResult(
                name=name,
                success=False,
                elapsed_seconds=elapsed,
                error=f"exit {result.returncode}: {stderr_tail}",
            )
        stdout_preview = (result.stdout or "")[:300]
        print(f"  OK in {elapsed:.1f}s")
        if stdout_preview:
            print(f"  STDOUT (head):\n{stdout_preview}")
        return StepResult(
            name=name,
            success=True,
            elapsed_seconds=elapsed,
            summary=stdout_preview,
        )
    except FileNotFoundError as exc:
        elapsed = time.monotonic() - start
        print(f"  FAIL (not found) in {elapsed:.1f}s: {exc}")
        return StepResult(
            name=name,
            success=False,
            elapsed_seconds=elapsed,
            error=str(exc),
        )
    except OSError as exc:
        elapsed = time.monotonic() - start
        print(f"  FAIL (os error) in {elapsed:.1f}s: {exc}")
        return StepResult(
            name=name,
            success=False,
            elapsed_seconds=elapsed,
            error=str(exc),
        )


# ---------------------------------------------------------------------------
# Pure functions (testable)
# ---------------------------------------------------------------------------


def source_tree_fingerprint(rust_root: Path, source_root: Path) -> str:
    """Hash every relative path and byte in a source tree deterministically.

    This mirrors ``generate_migration_manifest.source_tree_fingerprint``
    exactly so that disposition files validated by either script agree.
    """
    digest = hashlib.sha256()
    absolute_source_root = (rust_root / source_root).resolve()
    for path in sorted(absolute_source_root.rglob("*.rs")):
        relative = path.relative_to(rust_root.resolve()).as_posix()
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return f"sha256:{digest.hexdigest()}"


def worktree_path_for_sha(sha: str, worktree_root: Path) -> Path:
    """Return the deterministic worktree path for a given SHA."""
    return worktree_root / f"{WORKTREE_PREFIX}{sha[:SHORT_SHA_LEN]}"


def read_cargo_baseline_sha(cargo_toml: Path) -> str | None:
    """Extract ``java-baseline-commit`` from ``[workspace.metadata.qlexpress]``.

    Returns the SHA string, or ``None`` if the key is absent.
    """
    if not cargo_toml.is_file():
        return None
    text = cargo_toml.read_text(encoding="utf-8")
    in_section = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == "[workspace.metadata.qlexpress]":
            in_section = True
            continue
        if in_section and stripped.startswith("["):
            break
        if in_section and stripped.startswith("java-baseline-commit"):
            _, _, value = stripped.partition("=")
            return value.strip().strip('"').strip("'")
    return None


def validate_sha(sha: str) -> bool:
    """Return True if ``sha`` looks like a valid 40-hex git SHA."""
    if len(sha) != 40:
        return False
    try:
        int(sha, 16)
    except ValueError:
        return False
    return True


def count_states(rows: list[dict[str, Any]]) -> dict[str, int]:
    """Count state occurrences in a manifest section."""
    counts: dict[str, int] = defaultdict(int)
    for row in rows:
        counts[row.get("state", "UNKNOWN")] += 1
    return dict(sorted(counts.items()))


def format_state_table(title: str, counts: dict[str, int]) -> str:
    """Format state counts as a human-readable table."""
    total = sum(counts.values())
    lines = [f"\n  {title} ({total} total):"]
    for state, count in sorted(counts.items(), key=lambda x: -x[1]):
        pct = 100.0 * count / total if total else 0
        bar = "#" * min(count, 40)
        lines.append(f"    {state:<20s} {count:>5d}  ({pct:5.1f}%)  {bar}")
    return "\n".join(lines)


def format_owner_distribution(rows: list[dict[str, Any]]) -> str:
    """Show which Java owner types have the most unhandled methods."""
    owner_counts: dict[str, int] = defaultdict(int)
    for row in rows:
        owner = row.get("java_owner")
        if owner:
            name = owner.get("qualified_name", "unknown")
        else:
            name = row.get("java", {}).get("qualified_name", "unknown")
            # Extract type from qualified name
            if "::" in name:
                name = name.rsplit("::", 1)[0]
        owner_counts[name] += 1
    top = sorted(owner_counts.items(), key=lambda x: -x[1])[:15]
    lines = ["\n  Top owners with unhandled methods:"]
    for name, count in top:
        lines.append(f"    {name:<60s} {count:>4d}")
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Automate the UNVERIFIED upgrade workflow for qlexpress-rust.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Shared arguments
    shared = argparse.ArgumentParser(add_help=False)
    shared.add_argument(
        "--java-repo",
        type=Path,
        required=True,
        help="Path to the QLExpress Java repository.",
    )
    shared.add_argument(
        "--rust-root",
        type=Path,
        default=Path("."),
        help="Path to the qlexpress-rust repository (default: cwd).",
    )
    shared.add_argument(
        "--java-baseline-sha",
        type=str,
        required=True,
        help="40-hex Java commit SHA to use as baseline.",
    )
    shared.add_argument(
        "--worktree-root",
        type=Path,
        default=DEFAULT_WORKTREE_ROOT,
        help=f"Directory for temporary worktrees (default: {DEFAULT_WORKTREE_ROOT}).",
    )
    shared.add_argument(
        "--rust-source-root",
        type=Path,
        default=DEFAULT_RUST_SOURCE_ROOT,
        help=f"Rust source root relative to rust-root (default: {DEFAULT_RUST_SOURCE_ROOT}).",
    )
    shared.add_argument(
        "--java-package-root-suffix",
        type=str,
        default="src/main/java/com/alibaba/qlexpress4",
        help="Java package root path relative to worktree.",
    )

    # check
    check_parser = subparsers.add_parser(
        "check",
        parents=[shared],
        help="Read-only: report current unhandled method counts and owner distribution.",
    )

    # apply
    apply_parser = subparsers.add_parser(
        "apply",
        parents=[shared],
        help="Full upgrade: run all steps and commit results.",
    )
    apply_parser.add_argument(
        "--test-inventory-cmd",
        type=str,
        default=None,
        help=(
            "Shell command to generate the test inventory JSON. "
            "Receives --java-root and --rust-root as environment variables. "
            "If omitted, the existing test inventory is reused."
        ),
    )
    apply_parser.add_argument(
        "--skip-codegraph",
        action="store_true",
        help="Skip CodeGraph indexing (assume databases already exist).",
    )
    apply_parser.add_argument(
        "--skip-test-inventory",
        action="store_true",
        help="Skip test inventory generation (reuse existing file).",
    )

    # clean
    clean_parser = subparsers.add_parser(
        "clean",
        parents=[shared],
        help="Remove the worktree and any temporary files.",
    )

    return parser.parse_args(argv)


# ---------------------------------------------------------------------------
# Step implementations
# ---------------------------------------------------------------------------


def step_setup_worktree(cfg: UpgradeConfig) -> StepResult:
    """Create or reuse a git worktree for the Java baseline SHA."""
    if cfg.worktree_path.is_dir():
        # Verify it's at the correct SHA
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=cfg.worktree_path,
            capture_output=True,
            text=True,
        )
        if result.returncode == 0 and result.stdout.strip() == cfg.java_baseline_sha:
            print(f"  Worktree already exists at correct SHA: {cfg.worktree_path}")
            return StepResult(
                name="setup_worktree",
                success=True,
                elapsed_seconds=0,
                summary=f"Reused existing worktree: {cfg.worktree_path}",
            )
        # Wrong SHA — remove and recreate
        print(f"  Worktree exists but at wrong SHA, removing: {cfg.worktree_path}")
        run_step("remove_stale_worktree", ["rm", "-rf", str(cfg.worktree_path)])

    # Prune stale worktrees
    run_step(
        "prune_worktrees",
        ["git", "worktree", "prune"],
        cwd=cfg.java_repo,
        check=False,
    )

    return run_step(
        "setup_worktree",
        [
            "git", "worktree", "add",
            str(cfg.worktree_path),
            cfg.java_baseline_sha,
        ],
        cwd=cfg.java_repo,
    )


def step_index_codegraph(cfg: UpgradeConfig) -> StepResult:
    """Index both Java worktree and Rust tree with CodeGraph."""
    java_result = run_step(
        "index_java_codegraph",
        ["codegraph", "init", "-i", str(cfg.worktree_path)],
        cwd=cfg.worktree_path,
    )
    if not java_result.success:
        return java_result

    return run_step(
        "index_rust_codegraph",
        ["codegraph", "init", "-i", str(cfg.rust_root)],
        cwd=cfg.rust_root,
    )


def step_generate_test_inventory(cfg: UpgradeConfig) -> StepResult:
    """Generate or reuse the test inventory."""
    inventory_path = cfg.rust_root / "verification" / "migration-test-inventory-current.json"

    if cfg.test_inventory_cmd is None:
        if inventory_path.is_file():
            print(f"  Reusing existing test inventory: {inventory_path}")
            return StepResult(
                name="generate_test_inventory",
                success=True,
                elapsed_seconds=0,
                summary=f"Reused: {inventory_path}",
            )
        else:
            return StepResult(
                name="generate_test_inventory",
                success=False,
                elapsed_seconds=0,
                error=(
                    f"No test inventory at {inventory_path} and no "
                    "--test-inventory-cmd specified."
                ),
            )

    env = {
        "JAVA_ROOT": str(cfg.worktree_path),
        "RUST_ROOT": str(cfg.rust_root),
    }
    return run_step(
        "generate_test_inventory",
        ["sh", "-c", cfg.test_inventory_cmd],
        cwd=cfg.rust_root,
        env=env,
    )


def step_validate_fingerprint(cfg: UpgradeConfig) -> tuple[str, StepResult]:
    """Compute and validate the Rust source fingerprint.

    Returns ``(fingerprint, step_result)``.  On success the fingerprint
    matches the dispositions file; on failure the caller should abort.
    """
    start = time.monotonic()
    current_fingerprint = source_tree_fingerprint(cfg.rust_root, cfg.rust_source_root)

    if not cfg.dispositions_path.is_file():
        elapsed = time.monotonic() - start
        return current_fingerprint, StepResult(
            name="validate_fingerprint",
            success=False,
            elapsed_seconds=elapsed,
            error=f"Dispositions file not found: {cfg.dispositions_path}",
        )

    try:
        dispositions_raw = json.loads(
            cfg.dispositions_path.read_text(encoding="utf-8")
        )
    except (json.JSONDecodeError, OSError) as exc:
        elapsed = time.monotonic() - start
        return current_fingerprint, StepResult(
            name="validate_fingerprint",
            success=False,
            elapsed_seconds=elapsed,
            error=f"Cannot read dispositions: {exc}",
        )

    stored_fingerprint = dispositions_raw.get("rust_source_fingerprint")
    elapsed = time.monotonic() - start

    if stored_fingerprint == current_fingerprint:
        print(f"  Fingerprint matches: {current_fingerprint[:40]}...")
        return current_fingerprint, StepResult(
            name="validate_fingerprint",
            success=True,
            elapsed_seconds=elapsed,
            summary=f"Fingerprint OK: {current_fingerprint}",
        )

    # Fingerprint mismatch — provide detailed diagnostics
    stored_java = dispositions_raw.get("java_baseline", "<unknown>")
    error_lines = [
        "Rust source fingerprint mismatch.",
        f"  Dispositions file:  {cfg.dispositions_path}",
        f"  Stored fingerprint: {stored_fingerprint}",
        f"  Current fingerprint: {current_fingerprint}",
        f"  Stored java_baseline: {stored_java}",
        f"  Current java_baseline: {cfg.java_baseline_sha}",
        "",
        "This means the Rust source tree has changed since the dispositions",
        "were last validated.  Options:",
        "  1. Run the disposition validator to re-approve changed sources.",
        "  2. If the dispositions are truly stale, regenerate them from scratch.",
    ]
    return current_fingerprint, StepResult(
        name="validate_fingerprint",
        success=False,
        elapsed_seconds=elapsed,
        error="\n".join(error_lines),
    )


def step_generate_manifest(cfg: UpgradeConfig, skip_test_inventory: bool = False) -> StepResult:
    """Run generate_migration_manifest.py with the resolved paths."""
    manifest_script = cfg.rust_root / "scripts" / "generate_migration_manifest.py"
    if not manifest_script.is_file():
        return StepResult(
            name="generate_manifest",
            success=False,
            elapsed_seconds=0,
            error=f"Generator script not found: {manifest_script}",
        )

    # Use temp paths for output — only commit on success
    tmp_manifest = cfg.rust_root / "verification" / ".manifest-tmp.json"
    tmp_summary = cfg.rust_root / "verification" / ".manifest-summary-tmp.json"

    command = [
        sys.executable,
        str(manifest_script),
        "--java-root", str(cfg.worktree_path),
        "--rust-root", str(cfg.rust_root),
        "--java-package-root", str(cfg.java_package_root),
        "--rust-source-root", str(cfg.rust_source_root),
        "--dispositions", str(cfg.dispositions_path),
        "--output", str(tmp_manifest),
        "--summary-output", str(tmp_summary),
    ]

    inventory_path = cfg.rust_root / "verification" / "migration-test-inventory-current.json"
    if not skip_test_inventory and inventory_path.is_file():
        command.extend(["--test-inventory", str(inventory_path)])

    return run_step("generate_manifest", command, cwd=cfg.rust_root)


def step_commit_artifacts(cfg: UpgradeConfig) -> StepResult:
    """Move temp manifest files to committed locations."""
    tmp_manifest = cfg.rust_root / "verification" / ".manifest-tmp.json"
    tmp_summary = cfg.rust_root / "verification" / ".manifest-summary-tmp.json"

    if not tmp_manifest.is_file():
        return StepResult(
            name="commit_artifacts",
            success=False,
            elapsed_seconds=0,
            error=f"Temp manifest not found: {tmp_manifest}",
        )

    start = time.monotonic()
    try:
        shutil.move(str(tmp_manifest), str(cfg.manifest_current_path))
        if tmp_summary.is_file():
            shutil.move(str(tmp_summary), str(cfg.manifest_summary_path))
        elapsed = time.monotonic() - start
        return StepResult(
            name="commit_artifacts",
            success=True,
            elapsed_seconds=elapsed,
            summary=(
                f"Committed: {cfg.manifest_current_path.name}"
                + (
                    f", {cfg.manifest_summary_path.name}"
                    if cfg.manifest_summary_path.exists()
                    else ""
                )
            ),
        )
    except (OSError, shutil.Error) as exc:
        elapsed = time.monotonic() - start
        return StepResult(
            name="commit_artifacts",
            success=False,
            elapsed_seconds=elapsed,
            error=str(exc),
        )


def step_cleanup_temp(cfg: UpgradeConfig) -> StepResult:
    """Remove any leftover temp files in verification/."""
    start = time.monotonic()
    removed = []
    for name in (".manifest-tmp.json", ".manifest-summary-tmp.json"):
        path = cfg.rust_root / "verification" / name
        if path.is_file():
            path.unlink()
            removed.append(name)
    elapsed = time.monotonic() - start
    return StepResult(
        name="cleanup_temp",
        success=True,
        elapsed_seconds=elapsed,
        summary=f"Removed: {', '.join(removed)}" if removed else "Nothing to clean.",
    )


def step_remove_worktree(cfg: UpgradeConfig) -> StepResult:
    """Remove the worktree directory."""
    if not cfg.worktree_path.is_dir():
        return StepResult(
            name="remove_worktree",
            success=True,
            elapsed_seconds=0,
            summary="Worktree does not exist, nothing to remove.",
        )

    # Try git worktree remove first, fall back to rm -rf
    start = time.monotonic()
    result = subprocess.run(
        ["git", "worktree", "remove", "--force", str(cfg.worktree_path)],
        cwd=cfg.java_repo,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        # Fallback
        shutil.rmtree(cfg.worktree_path, ignore_errors=True)

    elapsed = time.monotonic() - start
    exists = cfg.worktree_path.is_dir()
    return StepResult(
        name="remove_worktree",
        success=not exists,
        elapsed_seconds=elapsed,
        summary=f"Removed: {cfg.worktree_path}" if not exists else "Failed to remove",
        error="" if not exists else f"Directory still exists: {cfg.worktree_path}",
    )


# ---------------------------------------------------------------------------
# Subcommand implementations
# ---------------------------------------------------------------------------


def cmd_check(cfg: UpgradeConfig) -> int:
    """Read-only check: report unhandled method counts and owner distribution."""
    print(f"\n{'#'*60}")
    print(f"  UNVERIFIED UPGRADE CHECK")
    print(f"  Java repo:    {cfg.java_repo}")
    print(f"  Rust root:    {cfg.rust_root}")
    print(f"  Baseline SHA: {cfg.java_baseline_sha}")
    print(f"  Worktree:     {cfg.worktree_path}")
    print(f"{'#'*60}")

    # Step 1: Verify Java repo exists
    if not cfg.java_repo.is_dir():
        print(f"\nERROR: Java repo not found: {cfg.java_repo}")
        return 1

    # Step 2: Verify worktree or set it up
    if not cfg.worktree_path.is_dir():
        print(f"\n  Worktree not found at {cfg.worktree_path}")
        print("  Setting up worktree (read-only check needs it for CodeGraph)...")
        result = step_setup_worktree(cfg)
        if not result.success:
            print(f"\nERROR: Failed to set up worktree: {result.error}")
            return 1

    # Step 3: Check CodeGraph databases
    if not cfg.java_db.is_file():
        print(f"\n  Java CodeGraph DB not found: {cfg.java_db}")
        print("  Indexing Java worktree...")
        result = run_step(
            "index_java_codegraph",
            ["codegraph", "init", "-i", str(cfg.worktree_path)],
            cwd=cfg.worktree_path,
        )
        if not result.success:
            print(f"\nERROR: CodeGraph indexing failed: {result.error}")
            return 1

    if not cfg.rust_db.is_file():
        print(f"\n  Rust CodeGraph DB not found: {cfg.rust_db}")
        print("  Indexing Rust tree...")
        result = run_step(
            "index_rust_codegraph",
            ["codegraph", "init", "-i", str(cfg.rust_root)],
            cwd=cfg.rust_root,
        )
        if not result.success:
            print(f"\nERROR: CodeGraph indexing failed: {result.error}")
            return 1

    # Step 4: Validate dispositions
    fingerprint, fp_result = step_validate_fingerprint(cfg)
    if not fp_result.success:
        print(f"\nWARNING: Fingerprint validation failed:")
        print(f"  {fp_result.error}")
        print("  Manifest generation would fail.  Check dispositions file.")

    # Step 5: Read current manifest summary (if available)
    if cfg.manifest_summary_path.is_file():
        try:
            summary = json.loads(
                cfg.manifest_summary_path.read_text(encoding="utf-8")
            )
            ms = summary.get("summary", {})
            print("\n  Current manifest summary:")
            for section in ("object_states", "nested_type_states", "method_states"):
                counts = ms.get(section, {})
                if counts:
                    print(format_state_table(section, counts))

            # Show UNVERIFIED count
            method_states = ms.get("method_states", {})
            unverified = method_states.get("UNVERIFIED", 0)
            missing = method_states.get("MISSING", 0)
            total = sum(method_states.values())
            handled = sum(
                v for k, v in method_states.items()
                if k in ("IMPLEMENTED", "DEPENDENCY_REUSED", "PLATFORM_NA")
            )
            print(f"\n  Summary: {handled}/{total} methods handled, "
                  f"{unverified} UNVERIFIED, {missing} MISSING")

            if unverified > 0 or missing > 0:
                # Try to show owner distribution from full manifest
                if cfg.manifest_current_path.is_file():
                    try:
                        manifest = json.loads(
                            cfg.manifest_current_path.read_text(encoding="utf-8")
                        )
                        unhandled = [
                            row for row in manifest.get("methods", [])
                            if row.get("state") not in (
                                "IMPLEMENTED", "DEPENDENCY_REUSED", "PLATFORM_NA"
                            )
                        ]
                        if unhandled:
                            print(format_owner_distribution(unhandled))
                    except (json.JSONDecodeError, OSError):
                        pass
        except (json.JSONDecodeError, OSError) as exc:
            print(f"\n  Could not read manifest summary: {exc}")
    else:
        print(f"\n  No manifest summary found at {cfg.manifest_summary_path}")
        print("  Run 'apply' first to generate the initial manifest.")

    # Step 6: Validate dispositions schema
    if cfg.dispositions_path.is_file():
        try:
            disp = json.loads(cfg.dispositions_path.read_text(encoding="utf-8"))
            sv = disp.get("schema_version")
            jb = disp.get("java_baseline", "")[:12]
            print(f"\n  Dispositions file: {cfg.dispositions_path}")
            print(f"    schema_version:     {sv}")
            print(f"    java_baseline:      {jb}...")
            print(f"    rust_source_fp:     {disp.get('rust_source_fingerprint', 'N/A')[:40]}...")
            for section in ("objects", "nested_java_types", "methods"):
                entries = disp.get(section, [])
                print(f"    {section}: {len(entries)} entries")
        except (json.JSONDecodeError, OSError) as exc:
            print(f"\n  Could not read dispositions: {exc}")
    else:
        print(f"\n  No dispositions file at {cfg.dispositions_path}")

    print(f"\n{'='*60}")
    print("  CHECK COMPLETE")
    print(f"{'='*60}")
    return 0


def cmd_apply(cfg: UpgradeConfig, *, skip_codegraph: bool = False,
              skip_test_inventory: bool = False) -> int:
    """Full upgrade: run all steps and commit results."""
    print(f"\n{'#'*60}")
    print(f"  UNVERIFIED UPGRADE APPLY")
    print(f"  Java repo:    {cfg.java_repo}")
    print(f"  Rust root:    {cfg.rust_root}")
    print(f"  Baseline SHA: {cfg.java_baseline_sha}")
    print(f"  Worktree:     {cfg.worktree_path}")
    print(f"{'#'*60}")

    steps: list[StepResult] = []
    total_start = time.monotonic()

    # Pre-flight: validate SHA
    if not validate_sha(cfg.java_baseline_sha):
        print(f"\nERROR: Invalid SHA: {cfg.java_baseline_sha}")
        return 1

    # Pre-flight: compute fingerprint BEFORE any mutations
    fingerprint, fp_result = step_validate_fingerprint(cfg)
    steps.append(fp_result)
    if not fp_result.success:
        print(f"\nABORT: Fingerprint validation failed.")
        print(f"  {fp_result.error}")
        print("\nNo files were modified.")
        return 1

    # Step 1: Worktree
    result = step_setup_worktree(cfg)
    steps.append(result)
    if not result.success:
        print(f"\nABORT: Worktree setup failed.")
        _print_summary(steps, total_start)
        return 1

    # Step 2: CodeGraph
    if not skip_codegraph:
        result = step_index_codegraph(cfg)
        steps.append(result)
        if not result.success:
            print(f"\nABORT: CodeGraph indexing failed.")
            _print_summary(steps, total_start)
            return 1

    # Step 3: Test inventory
    if not skip_test_inventory:
        result = step_generate_test_inventory(cfg)
        steps.append(result)
        if not result.success:
            print(f"\nABORT: Test inventory generation failed.")
            _print_summary(steps, total_start)
            return 1

    # Step 4: Generate manifest (to temp files)
    result = step_generate_manifest(cfg, skip_test_inventory=skip_test_inventory)
    steps.append(result)
    if not result.success:
        print(f"\nABORT: Manifest generation failed.")
        step_cleanup_temp(cfg)
        _print_summary(steps, total_start)
        return 1

    # Step 5: Commit artifacts
    result = step_commit_artifacts(cfg)
    steps.append(result)
    if not result.success:
        print(f"\nABORT: Failed to commit artifacts.")
        step_cleanup_temp(cfg)
        _print_summary(steps, total_start)
        return 1

    # Step 6: Cleanup temp
    result = step_cleanup_temp(cfg)
    steps.append(result)

    # Print before/after ledger
    _print_before_after(cfg, fingerprint)

    _print_summary(steps, total_start)
    return 0


def cmd_clean(cfg: UpgradeConfig) -> int:
    """Remove the worktree and any temporary files."""
    print(f"\n{'#'*60}")
    print(f"  UNVERIFIED UPGRADE CLEAN")
    print(f"  Worktree: {cfg.worktree_path}")
    print(f"{'#'*60}")

    steps: list[StepResult] = []
    total_start = time.monotonic()

    result = step_cleanup_temp(cfg)
    steps.append(result)

    result = step_remove_worktree(cfg)
    steps.append(result)

    _print_summary(steps, total_start)
    return 0 if all(s.success for s in steps) else 1


# ---------------------------------------------------------------------------
# Reporting helpers
# ---------------------------------------------------------------------------


def _print_before_after(cfg: UpgradeConfig, fingerprint: str) -> None:
    """Print the before/after ledger for the upgrade."""
    print(f"\n{'='*60}")
    print("  UPGRADE LEDGER")
    print(f"{'='*60}")

    # Before: read from the committed manifest (if it existed before)
    print(f"\n  Rust source fingerprint: {fingerprint[:50]}...")
    print(f"  Java baseline SHA:       {cfg.java_baseline_sha}")

    if cfg.manifest_summary_path.is_file():
        try:
            summary = json.loads(
                cfg.manifest_summary_path.read_text(encoding="utf-8")
            )
            ms = summary.get("summary", {})
            method_states = ms.get("method_states", {})
            object_states = ms.get("object_states", {})
            nested_states = ms.get("nested_type_states", {})

            print("\n  After upgrade:")
            print(format_state_table("Objects", object_states))
            print(format_state_table("Nested types", nested_states))
            print(format_state_table("Methods", method_states))

            unverified = method_states.get("UNVERIFIED", 0)
            missing = method_states.get("MISSING", 0)
            total = sum(method_states.values())
            handled = sum(
                v for k, v in method_states.items()
                if k in ("IMPLEMENTED", "DEPENDENCY_REUSED", "PLATFORM_NA")
            )
            print(f"\n  Result: {handled}/{total} methods handled, "
                  f"{unverified} UNVERIFIED, {missing} MISSING")
            if unverified == 0 and missing == 0:
                print("  ALL CLEAR: No unhandled methods remain.")
        except (json.JSONDecodeError, OSError) as exc:
            print(f"  Could not read summary: {exc}")


def _print_summary(steps: list[StepResult], total_start: float) -> None:
    """Print a summary table of all steps."""
    total_elapsed = time.monotonic() - total_start
    print(f"\n{'='*60}")
    print("  STEP SUMMARY")
    print(f"{'='*60}")
    print(f"  {'Step':<30s} {'Status':<8s} {'Time':>8s}")
    print(f"  {'-'*30} {'-'*8} {'-'*8}")
    for step in steps:
        status = "OK" if step.success else "FAIL"
        print(f"  {step.name:<30s} {status:<8s} {step.elapsed_seconds:>7.1f}s")
    print(f"  {'-'*30} {'-'*8} {'-'*8}")
    print(f"  {'TOTAL':<30s} {'':8s} {total_elapsed:>7.1f}s")
    print()


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> int:
    args = parse_args()

    cfg = UpgradeConfig(
        java_repo=args.java_repo,
        rust_root=args.rust_root,
        java_baseline_sha=args.java_baseline_sha,
        worktree_root=args.worktree_root,
        rust_source_root=args.rust_source_root,
        java_package_root_suffix=args.java_package_root_suffix,
        test_inventory_cmd=getattr(args, "test_inventory_cmd", None),
    )

    if args.command == "check":
        return cmd_check(cfg)
    elif args.command == "apply":
        return cmd_apply(
            cfg,
            skip_codegraph=getattr(args, "skip_codegraph", False),
            skip_test_inventory=getattr(args, "skip_test_inventory", False),
        )
    elif args.command == "clean":
        return cmd_clean(cfg)
    else:
        print(f"Unknown command: {args.command}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
