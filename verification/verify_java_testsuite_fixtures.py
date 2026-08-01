#!/usr/bin/env python3
"""Verify that Rust carries the same Java QL testsuite fixture contents."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


def fixture_bytes(path: Path) -> bytes:
    """Read one fixture without changing its encoding or line contents."""
    return path.read_bytes()


def inventory(root: Path) -> dict[str, bytes]:
    """Index every QL script by its testsuite-relative path and raw contents."""
    return {
        path.relative_to(root).as_posix(): fixture_bytes(path)
        for path in sorted(root.rglob("*.ql"))
    }


def logical_fixture_bytes(content: bytes) -> bytes:
    """Normalize the sole EOF-LF difference that the repository patch format cannot retain."""
    return content[:-1] if content.endswith(b"\n") else content


def main() -> int:
    parser = argparse.ArgumentParser(
        description="compare Java QL testsuite fixtures with Rust's vendored copy"
    )
    parser.add_argument("--java-repo", required=True, type=Path)
    parser.add_argument(
        "--rust-repo", type=Path, default=Path(__file__).resolve().parents[1]
    )
    args = parser.parse_args()

    java_root = args.java_repo.resolve() / "src/test/resources/testsuite"
    rust_root = args.rust_repo.resolve() / "crates/qlexpress/tests/fixtures/java-testsuite"
    if not java_root.is_dir():
        print(f"missing Java testsuite directory: {java_root}", file=sys.stderr)
        return 2
    if not rust_root.is_dir():
        print(f"missing Rust fixture directory: {rust_root}", file=sys.stderr)
        return 2

    java_files = inventory(java_root)
    rust_files = inventory(rust_root)
    missing = sorted(set(java_files) - set(rust_files))
    unexpected = sorted(set(rust_files) - set(java_files))
    changed = sorted(
        path
        for path in set(java_files) & set(rust_files)
        if logical_fixture_bytes(java_files[path]) != logical_fixture_bytes(rust_files[path])
    )
    eof_normalized = sorted(
        path
        for path in set(java_files) & set(rust_files)
        if java_files[path] != rust_files[path]
        and logical_fixture_bytes(java_files[path]) == logical_fixture_bytes(rust_files[path])
    )
    if missing or unexpected or changed:
        for label, paths in (("missing", missing), ("unexpected", unexpected), ("changed", changed)):
            for path in paths:
                print(f"{label}: {path}", file=sys.stderr)
        return 1

    print(
        f"fixtures match: {len(java_files)} QL scripts "
        f"(exact={len(java_files) - len(eof_normalized)}, eof-lf-normalized={len(eof_normalized)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
