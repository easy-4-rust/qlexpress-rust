#!/usr/bin/env python3
"""Generate an evidence-first Java-to-Rust migration manifest from CodeGraph.

The manifest deliberately does not infer semantic parity from names, file counts,
or green tests.  A discovered Rust candidate is ``UNVERIFIED`` until a reviewer
or a later verifier records contract-level evidence.  Structural gaps retain the
strict ``MISSING``/``MISPLACED``/``PARTIAL`` states required by the migration
process.
"""

from __future__ import annotations

import argparse
import json
import re
import sqlite3
import subprocess
from collections import defaultdict
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


JAVA_TYPE_KINDS = {"class", "interface", "enum"}
RUST_TYPE_KINDS = {"struct", "trait", "enum", "type_alias"}
PUBLIC_VISIBILITIES = {"public", "protected"}
STRICT_STATES = {
    "MISSING",
    "MISPLACED",
    "STUB",
    "PARTIAL",
    "UNVERIFIED",
    "IMPLEMENTED",
    "DEPENDENCY_REUSED",
    "PLATFORM_NA",
    "RUST_EXTENSION",
}
CHINESE = re.compile(r"[\u3400-\u9fff]")


@dataclass(frozen=True)
class Node:
    id: str
    kind: str
    name: str
    qualified_name: str
    file_path: str
    start_line: int
    end_line: int
    docstring: str
    signature: str
    visibility: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--java-root", type=Path, required=True)
    parser.add_argument("--rust-root", type=Path, required=True)
    parser.add_argument(
        "--java-package-root",
        type=Path,
        required=True,
        help="Java package root corresponding to the Rust crate's src directory.",
    )
    parser.add_argument(
        "--rust-source-root",
        type=Path,
        default=Path("crates/qlexpress/src"),
    )
    parser.add_argument(
        "--retain-segments",
        type=int,
        choices=(1, 2),
        default=1,
        help=(
            "Trailing Java package segments retained under Rust src. "
            "Repository policy requires 1."
        ),
    )
    parser.add_argument(
        "--test-inventory",
        type=Path,
        help="Optional JSON emitted by audit_migration_tests.py.",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--summary-output",
        type=Path,
        help="Optional compact, review-friendly manifest summary.",
    )
    return parser.parse_args()


def run(root: Path, *command: str) -> str:
    try:
        return subprocess.check_output(
            command,
            cwd=root,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "UNKNOWN"


def json_command(root: Path, *command: str) -> dict[str, Any] | str:
    value = run(root, *command)
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return value
    return parsed if isinstance(parsed, dict) else value


def git_state(root: Path) -> dict[str, Any]:
    status = run(root, "git", "status", "--porcelain=v1")
    return {
        "sha": run(root, "git", "rev-parse", "HEAD"),
        "branch": run(root, "git", "branch", "--show-current"),
        "dirty": status not in {"", "UNKNOWN"},
        "status": [] if status in {"", "UNKNOWN"} else status.splitlines(),
    }


def camel_to_snake(name: str) -> str:
    """Convert acronym-bearing Java names such as QLParser to ql_parser."""
    step_one = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return (
        re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", step_one)
        .replace("-", "_")
        .lower()
    )


def load_nodes(db_path: Path) -> list[Node]:
    connection = sqlite3.connect(db_path)
    connection.row_factory = sqlite3.Row
    try:
        rows = connection.execute(
            """
            SELECT id, kind, name, qualified_name, file_path, start_line, end_line,
                   COALESCE(docstring, '') AS docstring,
                   COALESCE(signature, '') AS signature,
                   COALESCE(visibility, '') AS visibility
            FROM nodes
            ORDER BY file_path, start_line, end_line
            """
        ).fetchall()
    finally:
        connection.close()
    return [Node(**dict(row)) for row in rows]


def direct_calls(db_path: Path, node_ids: set[str]) -> dict[str, list[dict[str, str]]]:
    if not node_ids:
        return {}
    connection = sqlite3.connect(db_path)
    connection.row_factory = sqlite3.Row
    calls: dict[str, list[dict[str, str]]] = defaultdict(list)
    try:
        for row in connection.execute(
            """
            SELECT e.source, target.kind, target.qualified_name, target.file_path,
                   target.start_line
            FROM edges e
            JOIN nodes target ON target.id = e.target
            WHERE e.kind = 'calls'
            ORDER BY e.source, target.qualified_name, target.start_line
            """
        ):
            if row["source"] not in node_ids:
                continue
            item = {
                "kind": row["kind"],
                "qualified_name": row["qualified_name"],
                "file": row["file_path"],
                "line": row["start_line"],
            }
            if item not in calls[row["source"]]:
                calls[row["source"]].append(item)
    finally:
        connection.close()
    return dict(calls)


def is_java_production(node: Node) -> bool:
    return node.file_path.startswith("src/main/java/") and node.file_path.endswith(
        ".java"
    )


def cfg_test_cutoffs(rust_root: Path, rust_source_root: Path) -> dict[str, int]:
    """Return the first module-level cfg(test) line for each Rust source file."""
    cutoffs: dict[str, int] = {}
    for path in (rust_root / rust_source_root).rglob("*.rs"):
        relative = path.relative_to(rust_root).as_posix()
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8", errors="replace").splitlines(),
            1,
        ):
            if re.match(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", line):
                cutoffs[relative] = line_number
                break
    return cutoffs


def is_rust_production(
    node: Node,
    rust_source_root: Path,
    test_cutoffs: dict[str, int],
) -> bool:
    prefix = rust_source_root.as_posix().rstrip("/") + "/"
    return (
        node.file_path.startswith(prefix)
        and node.file_path.endswith(".rs")
        and node.start_line < test_cutoffs.get(node.file_path, 2**31)
    )


def primary_java_types(nodes: list[Node]) -> list[Node]:
    """Return the file-owning Java object; nested declarations remain separate."""
    result = []
    for node in nodes:
        if node.kind not in JAVA_TYPE_KINDS or not is_java_production(node):
            continue
        if node.name.startswith("<"):
            continue
        if Path(node.file_path).stem == node.name:
            result.append(node)
    return result


def nested_java_types(nodes: list[Node], primary_ids: set[str]) -> list[Node]:
    return [
        node
        for node in nodes
        if node.kind in JAVA_TYPE_KINDS
        and is_java_production(node)
        and node.id not in primary_ids
        and not node.name.startswith("<")
    ]


def expected_rust_file(
    java_root: Path,
    package_root: Path,
    rust_source_root: Path,
    java_file: str,
    retain_segments: int,
) -> str:
    absolute = java_root / java_file
    package_parts = absolute.parent.relative_to(package_root).parts
    retained = package_parts[-retain_segments:]
    return (
        rust_source_root.joinpath(*retained, camel_to_snake(absolute.stem))
        .with_suffix(".rs")
        .as_posix()
    )


def nearest_owner(method: Node, types_by_file: dict[str, list[Node]]) -> Node | None:
    owners = [
        item
        for item in types_by_file.get(method.file_path, [])
        if item.start_line <= method.start_line <= item.end_line
    ]
    if not owners:
        return None
    return min(owners, key=lambda item: item.end_line - item.start_line)


def rust_test_markers(rust_root: Path) -> dict[str, list[dict[str, Any]]]:
    """Index explicit ``JavaClass#method`` markers located near Rust tests."""
    markers: dict[str, list[dict[str, Any]]] = defaultdict(list)
    marker_pattern = re.compile(r"\b([A-Za-z_$][A-Za-z0-9_$]*)#([A-Za-z_$][A-Za-z0-9_$]*)\b")
    for path in sorted((rust_root / "crates").rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        lines = text.splitlines()
        for index, line in enumerate(lines):
            for match in marker_pattern.finditer(line):
                nearby = "\n".join(lines[max(0, index - 4) : index + 18])
                test = re.search(
                    r"#\s*\[\s*test(?:\s*\([^]]*\))?\s*\][\s\S]*?"
                    r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
                    nearby,
                )
                if not test:
                    continue
                key = f"{match.group(1)}#{match.group(2)}"
                markers[key].append(
                    {
                        "file": path.relative_to(rust_root).as_posix(),
                        "line": index + 1,
                        "test": test.group(1),
                    }
                )
    return dict(markers)


def method_candidates(
    java_method: Node,
    rust_methods: list[Node],
    candidate_files: set[str],
) -> list[Node]:
    expected_name = camel_to_snake(java_method.name)
    names = {expected_name}
    if java_method.name == Path(java_method.file_path).stem:
        names.update({"new", "default", "with_init_options"})
    return [
        method
        for method in rust_methods
        if method.file_path in candidate_files and method.name in names
    ]


def node_dict(node: Node, include_doc: bool = True) -> dict[str, Any]:
    value: dict[str, Any] = {
        "id": node.id,
        "kind": node.kind,
        "name": node.name,
        "qualified_name": node.qualified_name,
        "file": node.file_path,
        "line": node.start_line,
        "end_line": node.end_line,
        "visibility": node.visibility,
        "signature": node.signature,
    }
    if include_doc:
        value["doc"] = node.docstring
    return value


def main() -> int:
    args = parse_args()
    java_root = args.java_root.resolve()
    rust_root = args.rust_root.resolve()
    package_root = args.java_package_root.resolve()
    rust_source_root = args.rust_source_root
    java_db = java_root / ".codegraph" / "codegraph.db"
    rust_db = rust_root / ".codegraph" / "codegraph.db"
    for path in (java_db, rust_db):
        if not path.is_file():
            raise SystemExit(f"missing CodeGraph database: {path}")

    java_nodes = load_nodes(java_db)
    rust_nodes = load_nodes(rust_db)
    primary_types = primary_java_types(java_nodes)
    primary_ids = {item.id for item in primary_types}
    nested_types = nested_java_types(java_nodes, primary_ids)
    anonymous_types = [
        node
        for node in java_nodes
        if node.kind in JAVA_TYPE_KINDS
        and is_java_production(node)
        and node.name.startswith("<")
    ]
    test_cutoffs = cfg_test_cutoffs(rust_root, rust_source_root)
    rust_types = [
        node
        for node in rust_nodes
        if node.kind in RUST_TYPE_KINDS
        and is_rust_production(node, rust_source_root, test_cutoffs)
    ]
    rust_methods = [
        node
        for node in rust_nodes
        if node.kind in {"method", "function"}
        and is_rust_production(node, rust_source_root, test_cutoffs)
    ]

    rust_files = {
        path.relative_to(rust_root).as_posix()
        for path in (rust_root / rust_source_root).rglob("*.rs")
    }
    rust_files_by_name: dict[str, list[str]] = defaultdict(list)
    for path in rust_files:
        rust_files_by_name[Path(path).name].append(path)
    rust_types_by_file: dict[str, list[Node]] = defaultdict(list)
    for node in rust_types:
        rust_types_by_file[node.file_path].append(node)

    java_types_by_file: dict[str, list[Node]] = defaultdict(list)
    for node in [*primary_types, *nested_types]:
        java_types_by_file[node.file_path].append(node)
    java_method_nodes = [
        node
        for node in java_nodes
        if node.kind == "method" and is_java_production(node)
    ]
    java_methods = []
    for node in java_method_nodes:
        owner = nearest_owner(node, java_types_by_file)
        if node.visibility in PUBLIC_VISIBILITIES or (
            owner is not None and owner.kind == "interface"
        ):
            java_methods.append(node)
    java_call_graph = direct_calls(java_db, {node.id for node in java_methods})
    test_markers = rust_test_markers(rust_root)

    object_rows: list[dict[str, Any]] = []
    files_for_java_object: dict[str, set[str]] = {}
    state_counts: dict[str, int] = defaultdict(int)
    for java_type in primary_types:
        expected_file = expected_rust_file(
            java_root,
            package_root,
            rust_source_root,
            java_type.file_path,
            args.retain_segments,
        )
        same_name_files = sorted(rust_files_by_name.get(Path(expected_file).name, []))
        if expected_file in rust_files:
            candidate_files = {expected_file}
            exact_types = [
                node
                for node in rust_types_by_file.get(expected_file, [])
                if node.name == java_type.name
            ]
            state = "UNVERIFIED" if exact_types else "PARTIAL"
        elif same_name_files:
            candidate_files = set(same_name_files)
            exact_types = [
                node
                for path in same_name_files
                for node in rust_types_by_file.get(path, [])
                if node.name == java_type.name
            ]
            state = "MISPLACED"
        else:
            candidate_files = set()
            exact_types = []
            state = "MISSING"
        assert state in STRICT_STATES
        state_counts[state] += 1
        files_for_java_object[java_type.id] = candidate_files
        object_rows.append(
            {
                "java": node_dict(java_type),
                "expected_rust_file": expected_file,
                "candidate_rust_files": sorted(candidate_files),
                "candidate_rust_types": [node_dict(node) for node in exact_types],
                "state": state,
                "semantic_evidence": [],
                "review_note": (
                    "候选仅按路径与类型名定位，尚未证明字段、方法、错误和副作用契约。"
                    if state == "UNVERIFIED"
                    else ""
                ),
            }
        )

    object_rows_by_file = {
        row["java"]["file"]: row
        for row in object_rows
    }
    nested_rows: list[dict[str, Any]] = []
    nested_state_counts: dict[str, int] = defaultdict(int)
    for nested_type in nested_types:
        parent_row = object_rows_by_file[nested_type.file_path]
        candidate_files = set(parent_row["candidate_rust_files"])
        exact_types = [
            node
            for path in candidate_files
            for node in rust_types_by_file.get(path, [])
            if node.name == nested_type.name
        ]
        if exact_types:
            state = (
                "MISPLACED"
                if parent_row["state"] == "MISPLACED"
                else "UNVERIFIED"
            )
        else:
            state = "MISSING"
        nested_state_counts[state] += 1
        nested_rows.append(
            {
                "java": node_dict(nested_type),
                "owning_java_object": parent_row["java"]["qualified_name"],
                "candidate_rust_types": [node_dict(node) for node in exact_types],
                "state": state,
                "semantic_evidence": [],
                "review_note": (
                    "内部类型仍需独立核对字段、构造和行为契约。"
                    if exact_types
                    else "未在所属 Rust 对象文件中发现同名内部类型。"
                ),
            }
        )

    primary_by_file = {item.file_path: item for item in primary_types}
    method_rows: list[dict[str, Any]] = []
    method_state_counts: dict[str, int] = defaultdict(int)
    for java_method in java_methods:
        owner = nearest_owner(java_method, java_types_by_file)
        primary = primary_by_file.get(java_method.file_path)
        candidate_files = (
            files_for_java_object.get(primary.id, set()) if primary else set()
        )
        candidates = method_candidates(java_method, rust_methods, candidate_files)
        state = "UNVERIFIED" if candidates else "MISSING"
        method_state_counts[state] += 1
        source_class = Path(java_method.file_path).stem
        marker_key = f"{source_class}#{java_method.name}"
        method_rows.append(
            {
                "java": node_dict(java_method),
                "java_owner": node_dict(owner) if owner else None,
                "direct_java_calls": java_call_graph.get(java_method.id, []),
                "candidate_rust_methods": [
                    node_dict(candidate) for candidate in candidates
                ],
                "test_evidence": test_markers.get(marker_key, []),
                "state": state,
                "semantic_evidence": [],
                "review_note": (
                    "同名候选不证明重载、参数、返回值、错误和副作用等价。"
                    if state == "UNVERIFIED"
                    else "未发现名称级 Rust 候选；需要人工确认适配名或补齐实现。"
                ),
            }
        )

    java_state = git_state(java_root)
    rust_state = git_state(rust_root)
    object_candidate_docs = [
        candidate["doc"]
        for row in object_rows
        for candidate in row["candidate_rust_types"]
    ]
    method_candidate_docs = [
        candidate["doc"]
        for row in method_rows
        for candidate in row["candidate_rust_methods"]
    ]
    comment_coverage = {
        "java_primary_objects_with_doc": sum(
            bool(row["java"]["doc"].strip()) for row in object_rows
        ),
        "rust_object_candidates": len(object_candidate_docs),
        "rust_object_candidates_with_chinese_doc": sum(
            bool(CHINESE.search(doc)) for doc in object_candidate_docs
        ),
        "rust_object_candidates_with_java_source_trace": sum(
            "对应 Java" in doc for doc in object_candidate_docs
        ),
        "java_methods_with_doc": sum(
            bool(row["java"]["doc"].strip()) for row in method_rows
        ),
        "rust_method_candidates": len(method_candidate_docs),
        "rust_method_candidates_with_chinese_doc": sum(
            bool(CHINESE.search(doc)) for doc in method_candidate_docs
        ),
        "rust_method_candidates_with_java_source_trace": sum(
            "对应 Java" in doc for doc in method_candidate_docs
        ),
    }
    evidence_coverage = {
        "java_methods_with_name_candidate": sum(
            bool(row["candidate_rust_methods"]) for row in method_rows
        ),
        "java_methods_with_explicit_test_marker": sum(
            bool(row["test_evidence"]) for row in method_rows
        ),
        "java_methods_with_direct_call_graph": sum(
            bool(row["direct_java_calls"]) for row in method_rows
        ),
        "direct_java_call_edges_recorded": sum(
            len(row["direct_java_calls"]) for row in method_rows
        ),
    }
    manifest = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "policy": {
            "source": "rust-java-migration + rust-java-migration-testing",
            "retain_package_segments": args.retain_segments,
            "candidate_rule": (
                "name/path matches are discovery evidence only; semantic parity "
                "requires explicit contract evidence"
            ),
            "strict_states": sorted(STRICT_STATES),
        },
        "baselines": {
            "java": {
                "root": str(java_root),
                **java_state,
                "codegraph": json_command(
                    java_root, "codegraph", "status", "--json"
                ),
            },
            "rust": {
                "root": str(rust_root),
                **rust_state,
                "codegraph": json_command(
                    rust_root, "codegraph", "status", "--json"
                ),
            },
        },
        "summary": {
            "java_primary_objects": len(primary_types),
            "java_nested_types": len(nested_types),
            "java_anonymous_types": len(anonymous_types),
            "java_public_or_protected_methods": len(java_methods),
            "rust_production_types": len(rust_types),
            "rust_production_methods": len(rust_methods),
            "object_states": dict(sorted(state_counts.items())),
            "nested_type_states": dict(sorted(nested_state_counts.items())),
            "method_states": dict(sorted(method_state_counts.items())),
            "explicit_test_marker_keys": len(test_markers),
            "comment_coverage": comment_coverage,
            "evidence_coverage": evidence_coverage,
        },
        "objects": object_rows,
        "nested_java_types": nested_rows,
        "anonymous_java_types": [
            {
                "java": node_dict(node),
                "state": "UNVERIFIED",
                "review_note": (
                    "匿名 Java 类型不要求独立 Rust 文件，但其可观察行为必须由所属"
                    "方法的语义映射与测试证明。"
                ),
            }
            for node in anonymous_types
        ],
        "methods": method_rows,
    }
    if args.test_inventory:
        test_inventory = json.loads(
            args.test_inventory.read_text(encoding="utf-8")
        )
        manifest["summary"]["tests"] = test_inventory["summary"]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    if args.summary_output:
        compact = {
            "schema_version": manifest["schema_version"],
            "generated_at": manifest["generated_at"],
            "policy": manifest["policy"],
            "baselines": manifest["baselines"],
            "summary": manifest["summary"],
            "artifacts": {
                "generator": "scripts/generate_migration_manifest.py",
                "full_manifest": str(args.output),
                "test_inventory": (
                    str(args.test_inventory) if args.test_inventory else None
                ),
            },
            "completion": {
                "proven": False,
                "reason": (
                    "严格对象、内部类型和方法状态尚未清零；"
                    "UNVERIFIED 也不计作 IMPLEMENTED。"
                ),
            },
        }
        args.summary_output.parent.mkdir(parents=True, exist_ok=True)
        args.summary_output.write_text(
            json.dumps(compact, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
    print(json.dumps(manifest["summary"], ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
