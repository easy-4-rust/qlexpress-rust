#!/usr/bin/env python3
"""Build the pinned Java baseline and compare Java/Rust execution records."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


def run(command: list[str], cwd: Path) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def load_records(path: Path) -> dict[str, dict[str, object]]:
    records: dict[str, dict[str, object]] = {}
    with path.open(encoding="utf-8") as source:
        for line in source:
            if line.strip():
                record = json.loads(line)
                records[str(record["id"])] = record
    return records


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--java-repo", required=True, type=Path)
    parser.add_argument(
        "--corpus",
        type=Path,
        default=Path("verification/corpus/differential.jsonl"),
    )
    args = parser.parse_args()

    repository = Path(__file__).resolve().parents[1]
    java_repo = args.java_repo.resolve()
    corpus = (repository / args.corpus).resolve()
    java_project = repository / "verification/java"
    output = repository / "target/verification"
    output.mkdir(parents=True, exist_ok=True)
    java_output = output / "java-differential.jsonl"
    rust_output = output / "rust-differential.jsonl"

    run(["mvn", "-q", "-DskipTests", "install"], java_repo)
    run(
        [
            "mvn",
            "-q",
            "package",
            "dependency:build-classpath",
            "-Dmdep.outputFile=target/classpath.txt",
        ],
        java_project,
    )
    classpath_file = java_project / "target/classpath.txt"
    classpath = os.pathsep.join(
        [str(java_project / "target/classes"), classpath_file.read_text().strip()]
    )
    run(
        [
            "java",
            "-cp",
            classpath,
            "com.easy4rust.qlexpress.JavaDifferentialRunner",
            str(corpus),
            str(java_output),
        ],
        repository,
    )
    run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "qlexpress-verification",
            "--",
            "differential",
            str(corpus),
            str(rust_output),
        ],
        repository,
    )

    java_records = load_records(java_output)
    rust_records = load_records(rust_output)
    missing = sorted(set(java_records) ^ set(rust_records))
    mismatches = []
    for case_id in sorted(set(java_records) & set(rust_records)):
        if java_records[case_id] != rust_records[case_id]:
            mismatches.append(
                {
                    "id": case_id,
                    "java": java_records[case_id],
                    "rust": rust_records[case_id],
                }
            )
    report = {
        "cases": len(java_records),
        "matched": len(java_records) - len(mismatches) - len(missing),
        "mismatches": mismatches,
        "missing": missing,
    }
    report_path = output / "differential-report.json"
    report_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(report, ensure_ascii=False))
    if missing or mismatches:
        print(f"differential report: {report_path}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
