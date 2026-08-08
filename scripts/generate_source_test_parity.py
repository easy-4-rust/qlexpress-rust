#!/usr/bin/env python3
"""Generate the strict source-test and byte-identical asset parity manifest.

The generator deliberately keeps behavioral fields unverified until the whole-project
runner records Java, Rust, and per-case differential artifacts. It never promotes a
name mapping or two green suites to result parity.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def git_sha(root: Path) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True
    ).strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def relative_to_manifest(path: Path, manifest: Path) -> str:
    return path.resolve().relative_to(manifest.parent.resolve()).as_posix()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--java-root", type=Path, required=True)
    parser.add_argument("--rust-root", type=Path, default=Path("."))
    parser.add_argument(
        "--mapping", type=Path, default=Path("verification/migration-manifest.json")
    )
    parser.add_argument(
        "--output", type=Path, default=Path("docs/source-test-parity.json")
    )
    args = parser.parse_args()

    java_root = args.java_root.resolve()
    rust_root = args.rust_root.resolve()
    mapping = json.loads((rust_root / args.mapping).read_text(encoding="utf-8"))
    output = (rust_root / args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)

    source_tests = []
    for row in mapping["java_test_mappings"]:
        java = row["java"]
        targets = [
            f"{item['file']}#{item['test']}" for item in row["rust_test_evidence"]
        ]
        source_tests.append(
            {
                "source": f"{java['file']}#{java['name']}",
                "case_id": "default",
                "targets": targets,
                "disposition": "MIRRORED" if targets else "MISSING",
                "contract_preserved": False,
                "inputs_preserved": False,
                "assertions_preserved": False,
                "fixture_state_preserved": False,
                "cleanup_preserved": False,
                "case_expansion_complete": java["kind"] == "Test",
                "result_parity": "NOT_RUN",
                "evidence": "verification/runs/source-case-differential.json",
            }
        )

    java_assets_root = java_root / "src/test/resources"
    rust_assets_root = (
        rust_root / "qlexpress-test/tests/suite/source/src/test/resources"
    )
    assets = []
    for source in sorted(path for path in java_assets_root.rglob("*") if path.is_file()):
        relative = source.relative_to(java_root)
        target = rust_assets_root / source.relative_to(java_assets_root)
        assets.append(
            {
                "source": relative.as_posix(),
                "target": target.relative_to(rust_root).as_posix(),
                "mode": "COPY_EXACT",
                "sha256": sha256(source),
                "target_sha256": sha256(target) if target.is_file() else None,
            }
        )

    document = {
        "schema": 1,
        "java_baseline": git_sha(java_root),
        "rust_baseline": git_sha(rust_root),
        "acceptance_module": {
            "package": "qlexpress-test",
            "manifest": "qlexpress-test/Cargo.toml",
            "publish": False,
            "components": [
                "qlexpress",
                "qlexpress-derive",
                "qlexpress-process",
                "qlexpress-verification",
            ],
            "command": "cargo test -p qlexpress-test --all-features",
            "status": "NOT_RUN",
            "failed": 0,
            "skipped": 0,
            "not_run": len(source_tests),
            "artifact": "verification/runs/whole-project.json",
        },
        "source_tests": source_tests,
        "assets": assets,
        "runs": {
            "java": {
                "command": "JAVA_HOME=<jdk17> mvn test",
                "status": "NOT_RUN",
                "failed": 0,
                "skipped": 0,
                "not_run": len(source_tests),
                "artifact": "verification/runs/java.json",
            },
            "rust": {
                "command": "cargo test --workspace --all-features",
                "status": "NOT_RUN",
                "failed": 0,
                "skipped": 0,
                "not_run": len(source_tests),
                "artifact": "verification/runs/rust.json",
            },
            "differential": {
                "command": (
                    "python3 verification/run_differential.py --java-repo "
                    "/Users/wandl/workspaces/workspace-github/QLExpress"
                ),
                "status": "NOT_RUN",
                "matched": 0,
                "mismatched": 0,
                "harness_failures": 0,
                "not_run": len(source_tests),
                "artifact": "verification/runs/source-case-differential.json",
            },
        },
    }
    output.write_text(
        json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    exact = sum(row["sha256"] == row["target_sha256"] for row in assets)
    print(
        f"wrote {relative_to_manifest(output, output)}: "
        f"tests={len(source_tests)}, exact_assets={exact}/{len(assets)}"
    )
    return 0 if exact == len(assets) else 1


if __name__ == "__main__":
    raise SystemExit(main())
