#!/usr/bin/env python3
"""Generate an evidence-first Java-to-Rust migration manifest from CodeGraph.

The manifest deliberately does not infer semantic parity from names, file counts,
or green tests.  A discovered Rust candidate is ``UNVERIFIED`` until a reviewer
or a later verifier records contract-level evidence.  Structural gaps retain the
strict ``MISSING``/``MISPLACED``/``PARTIAL`` states required by the migration
process.  Reviewed states may only be supplied through a baseline-pinned
disposition file whose source and test anchors are validated against the
current checkout.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sqlite3
import subprocess
from collections import defaultdict
from dataclasses import dataclass, replace
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
HANDLED_STATES = {"IMPLEMENTED", "DEPENDENCY_REUSED", "PLATFORM_NA"}

# Java types from ANTLR4 runtime that are not applicable to the Rust custom parser.
# Methods on these types will be auto-marked PLATFORM_NA.
ANTLR_FRAMEWORK_TYPES = {
    "com.alibaba.qlexpress4.aparser::RuleContext",
    "com.alibaba.qlexpress4.aparser::Token",
}

# Java types whose methods are reflection-based utilities with no Rust equivalent.
REFLECTION_NA_TYPES = {
    "com.alibaba.qlexpress4.utils::BasicUtil#isPublic",
    "com.alibaba.qlexpress4.utils::BasicUtil#isStatic",
}

# Java inner record types handled differently in Rust (fields or derive macros).
# Methods on these types will be auto-marked RUST_EXTENSION when the Rust owner
# type has matching derive macros.
DERIVE_HANDLED_TYPES = {
    "com.alibaba.qlexpress4.runtime::MetaClass",
    "com.alibaba.qlexpress4.runtime::ReflectLoader::ExtensionMapKey",
    "com.alibaba.qlexpress4.runtime::ReflectLoader::MethodCacheKey",
}

# Java method names that are provided by Rust derive macros.
DERIVE_METHOD_NAMES = {"equals", "hashCode", "toString"}

# Java methods whose Rust equivalents live in a different architectural location.
# These are genuine Rust design decisions, not missing implementations.
# Key: (java_owner_qualified_name, java_method_name) → state
KNOWN_RELOCATIONS = {
    "com.alibaba.qlexpress4.aparser::QvmInstructionVisitor#visitArrayInitializer": "RUST_EXTENSION",
    "com.alibaba.qlexpress4.aparser::QvmInstructionVisitor#visitConstExpr": "RUST_EXTENSION",
    "com.alibaba.qlexpress4.aparser::SyntaxTreeFactory#buildTree": "RUST_EXTENSION",
    "com.alibaba.qlexpress4.proxy::QLambdaInvocationHandler#invoke": "RUST_EXTENSION",
}

# ReflectLoader inner record getter methods - in Rust these types don't exist
# as separate records; caching uses different architecture.
REFLECT_LOADER_INNER_GETTERS = {
    "getCls", "getMethodName", "getArgTypes",
}

# Cross-type delegation map: Java owner qualified_name → set of Rust qualified_name
# prefixes where the methods actually live.  Used when Rust splits a Java interface
# implementation across multiple types (e.g. QvmBlockScope delegates to QScope).
CROSS_TYPE_DELEGATION: dict[str, set[str]] = {
    "com.alibaba.qlexpress4.runtime.scope::QvmBlockScope": {"QScope"},
    "com.alibaba.qlexpress4.runtime::QvmGlobalScope": {"QScope", "QvmGlobalScope"},
    "com.alibaba.qlexpress4.runtime::ReflectLoader::ExtensionMapKey": {"ReflectLoader"},
    "com.alibaba.qlexpress4.runtime::ReflectLoader::MethodCacheKey": {"ReflectLoader"},
}
DISPOSITION_CLASSIFICATIONS = {
    "EXACT",
    "ADAPTED",
    "DEPENDENCY_REUSED",
    "PLATFORM_NA",
}
TEST_EVIDENCE_LEVELS = {
    "V1_RUST_LOCAL",
    "V2_MIRRORED",
    "V3_GOLDEN_DIFF",
    "V4_LIVE_DIFF",
    "V5_HOST",
    "V6_NONFUNCTIONAL",
    "V7_ROLLBACK",
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
    parser.add_argument(
        "--dispositions",
        type=Path,
        help=(
            "Optional baseline-pinned JSON containing manually reviewed object, "
            "nested-type, method, and source-test dispositions."
        ),
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


def dirty_paths(state: dict[str, Any]) -> list[str]:
    """Extract repository-relative paths from porcelain-v1 status rows."""
    paths: list[str] = []
    for row in state["status"]:
        value = row[3:]
        if " -> " in value:
            value = value.split(" -> ", 1)[1]
        paths.append(value)
    return paths


def count_states(rows: list[dict[str, Any]]) -> dict[str, int]:
    """Count strict states after reviewed dispositions have been applied."""
    counts: dict[str, int] = defaultdict(int)
    for row in rows:
        counts[row["state"]] += 1
    return counts


def source_tree_fingerprint(root: Path, source_root: Path) -> str:
    """Hash every relative path and byte in a source tree deterministically."""
    digest = hashlib.sha256()
    absolute_source_root = (root / source_root).resolve()
    for path in sorted(absolute_source_root.rglob("*.rs")):
        relative = path.relative_to(root.resolve()).as_posix()
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return f"sha256:{digest.hexdigest()}"


def git_is_ancestor(root: Path, ancestor: str, descendant: str) -> bool:
    """Return whether a reviewed baseline is an ancestor of the checkout."""
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def normalized_signature(signature: str) -> str:
    """Return a stable, whitespace-normalized CodeGraph signature."""
    return " ".join(signature.split())


def java_key(java: dict[str, Any]) -> str:
    """Build the stable disposition key for one Java source node."""
    return (
        f"{java['qualified_name']}|"
        f"{normalized_signature(str(java.get('signature', '')))}"
    )


def _require_non_empty_string(
    value: Any,
    *,
    field: str,
    java_key_value: str,
) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(
            f"disposition {java_key_value}: {field} must be a non-empty string"
        )
    return value.strip()


def _validate_file_anchor(
    anchor: Any,
    *,
    rust_root: Path,
    java_key_value: str,
    field: str,
    symbol_field: str,
) -> dict[str, str]:
    if not isinstance(anchor, dict):
        raise ValueError(
            f"disposition {java_key_value}: {field} entries must be objects"
        )
    file_name = _require_non_empty_string(
        anchor.get("file"),
        field=f"{field}.file",
        java_key_value=java_key_value,
    )
    symbol = _require_non_empty_string(
        anchor.get(symbol_field),
        field=f"{field}.{symbol_field}",
        java_key_value=java_key_value,
    )
    resolved_rust_root = rust_root.resolve()
    path = (resolved_rust_root / file_name).resolve()
    try:
        path.relative_to(resolved_rust_root)
    except ValueError as error:
        raise ValueError(
            f"disposition {java_key_value}: {field}.file escapes Rust root: "
            f"{file_name}"
        ) from error
    if not path.is_file():
        raise ValueError(
            f"disposition {java_key_value}: evidence file does not exist: "
            f"{file_name}"
        )
    source = path.read_text(encoding="utf-8", errors="replace")
    found = symbol in source
    # Also check include!-reachable files
    if not found:
        import re as _re
        for _inc in _re.finditer(r'include!\("([^"]+)"\)', source):
            _inc_path = path.parent / _inc.group(1)
            if _inc_path.is_file():
                _inc_source = _inc_path.read_text(encoding="utf-8", errors="replace")
                if symbol in _inc_source:
                    found = True
                    break
    if not found:
        raise ValueError(
            f"disposition {java_key_value}: {field}.{symbol_field} "
            f"{symbol!r} not found in {file_name}"
        )
    return {"file": file_name, symbol_field: symbol}


def validate_disposition(
    raw: Any,
    *,
    rust_root: Path,
) -> dict[str, Any]:
    """Validate one reviewed parity disposition and all local evidence anchors."""
    if not isinstance(raw, dict):
        raise ValueError("disposition entries must be JSON objects")
    key = _require_non_empty_string(
        raw.get("java_key"),
        field="java_key",
        java_key_value="<unknown>",
    )
    state = _require_non_empty_string(
        raw.get("state"),
        field="state",
        java_key_value=key,
    )
    if state not in STRICT_STATES - {"RUST_EXTENSION"}:
        raise ValueError(f"disposition {key}: unsupported state {state!r}")
    classification = _require_non_empty_string(
        raw.get("classification"),
        field="classification",
        java_key_value=key,
    )
    if classification not in DISPOSITION_CLASSIFICATIONS:
        raise ValueError(
            f"disposition {key}: unsupported classification {classification!r}"
        )
    if state == "DEPENDENCY_REUSED" and classification != "DEPENDENCY_REUSED":
        raise ValueError(
            f"disposition {key}: DEPENDENCY_REUSED requires matching classification"
        )
    if state == "PLATFORM_NA" and classification != "PLATFORM_NA":
        raise ValueError(
            f"disposition {key}: PLATFORM_NA requires matching classification"
        )
    if state == "IMPLEMENTED" and classification not in {"EXACT", "ADAPTED"}:
        raise ValueError(
            f"disposition {key}: IMPLEMENTED requires EXACT or ADAPTED"
        )

    semantic_evidence = raw.get("semantic_evidence", [])
    if not isinstance(semantic_evidence, list) or any(
        not isinstance(item, str) or not item.strip()
        for item in semantic_evidence
    ):
        raise ValueError(
            f"disposition {key}: semantic_evidence must contain non-empty strings"
        )
    review_note = _require_non_empty_string(
        raw.get("review_note"),
        field="review_note",
        java_key_value=key,
    )

    raw_rust_evidence = raw.get("rust_evidence", [])
    if not isinstance(raw_rust_evidence, list):
        raise ValueError(
            f"disposition {key}: rust_evidence must be an array"
        )
    rust_evidence = [
        _validate_file_anchor(
            item,
            rust_root=rust_root,
            java_key_value=key,
            field="rust_evidence",
            symbol_field="symbol",
        )
        for item in raw_rust_evidence
    ]
    raw_test_evidence = raw.get("test_evidence", [])
    if not isinstance(raw_test_evidence, list):
        raise ValueError(
            f"disposition {key}: test_evidence must be an array"
        )
    test_evidence = []
    for item in raw_test_evidence:
        validated = _validate_file_anchor(
            item,
            rust_root=rust_root,
            java_key_value=key,
            field="test_evidence",
            symbol_field="test",
        )
        level = _require_non_empty_string(
            item.get("level") if isinstance(item, dict) else None,
            field="test_evidence.level",
            java_key_value=key,
        )
        if level not in TEST_EVIDENCE_LEVELS:
            raise ValueError(
                f"disposition {key}: unsupported evidence level {level!r}"
            )
        validated["level"] = level
        test_evidence.append(validated)

    if state in {"IMPLEMENTED", "DEPENDENCY_REUSED"}:
        if not semantic_evidence:
            raise ValueError(
                f"disposition {key}: {state} requires semantic_evidence"
            )
        if not rust_evidence:
            raise ValueError(
                f"disposition {key}: {state} requires rust_evidence"
            )
        if not test_evidence:
            raise ValueError(
                f"disposition {key}: {state} requires test_evidence"
            )
    platform_evidence = raw.get("platform_evidence", [])
    if state == "PLATFORM_NA":
        if (
            not isinstance(platform_evidence, list)
            or not platform_evidence
            or any(
                not isinstance(item, str) or not item.strip()
                for item in platform_evidence
            )
        ):
            raise ValueError(
                f"disposition {key}: PLATFORM_NA requires platform_evidence"
            )

    dependency_evidence = raw.get("dependency_evidence")
    if state == "DEPENDENCY_REUSED":
        if not isinstance(dependency_evidence, dict):
            raise ValueError(
                f"disposition {key}: DEPENDENCY_REUSED requires dependency_evidence"
            )
        for field in ("package", "version_or_commit", "upstream_symbol", "adapter"):
            _require_non_empty_string(
                dependency_evidence.get(field),
                field=f"dependency_evidence.{field}",
                java_key_value=key,
            )

    return {
        **raw,
        "java_key": key,
        "state": state,
        "classification": classification,
        "semantic_evidence": [item.strip() for item in semantic_evidence],
        "review_note": review_note,
        "rust_evidence": rust_evidence,
        "test_evidence": test_evidence,
    }


def load_dispositions(
    path: Path,
    *,
    java_sha: str,
    rust_sha: str,
    rust_source_fingerprint: str,
    rust_root: Path,
) -> dict[str, dict[str, dict[str, Any]]]:
    """Load and baseline-check the authoritative reviewed disposition file."""
    raw = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict) or raw.get("schema_version") != 1:
        raise ValueError("dispositions must use schema_version 1")
    if raw.get("java_baseline") != java_sha:
        raise ValueError(
            "disposition Java baseline does not match the current Java checkout"
        )
    reviewed_rust_baseline = raw.get("rust_baseline")
    if (
        reviewed_rust_baseline != rust_sha
        and (
            not isinstance(reviewed_rust_baseline, str)
            or not git_is_ancestor(rust_root, reviewed_rust_baseline, rust_sha)
        )
    ):
        raise ValueError(
            "disposition Rust baseline is not the current checkout or one of "
            "its ancestors"
        )
    if raw.get("rust_source_fingerprint") != rust_source_fingerprint:
        raise ValueError(
            "disposition Rust source fingerprint does not match current sources"
        )

    result: dict[str, dict[str, dict[str, Any]]] = {}
    for section in ("objects", "nested_java_types", "methods"):
        entries = raw.get(section, [])
        if not isinstance(entries, list):
            raise ValueError(f"dispositions section {section} must be an array")
        indexed: dict[str, dict[str, Any]] = {}
        for entry in entries:
            if not isinstance(entry, dict):
                raise ValueError(
                    f"dispositions section {section} entries must be objects"
                )
            has_key = "java_key" in entry
            has_keys = "java_keys" in entry
            if has_key == has_keys:
                raise ValueError(
                    f"disposition in {section} must define exactly one of "
                    "java_key or java_keys"
                )
            keys = entry.get("java_keys") if has_keys else [entry.get("java_key")]
            if (
                not isinstance(keys, list)
                or not keys
                or any(not isinstance(key, str) or not key.strip() for key in keys)
            ):
                raise ValueError(
                    f"disposition in {section}: java_keys must contain "
                    "non-empty strings"
                )
            for raw_key in keys:
                expanded = {
                    key: value
                    for key, value in entry.items()
                    if key != "java_keys"
                }
                expanded["java_key"] = raw_key
                validated = validate_disposition(expanded, rust_root=rust_root)
                key = validated["java_key"]
                if key in indexed:
                    raise ValueError(
                        f"duplicate disposition key in {section}: {key}"
                    )
                indexed[key] = validated
        result[section] = indexed
    return result


def apply_dispositions(
    rows: list[dict[str, Any]],
    dispositions: dict[str, dict[str, Any]],
    *,
    section: str,
) -> dict[str, int]:
    """Apply only exact reviewed keys and reject stale/unmatched dispositions."""
    matched: set[str] = set()
    for row in rows:
        key = java_key(row["java"])
        disposition = dispositions.get(key)
        row["java_key"] = key
        if disposition is None:
            continue
        matched.add(key)
        row["state"] = disposition["state"]
        row["semantic_evidence"] = disposition["semantic_evidence"]
        row["review_note"] = disposition["review_note"]
        row["reviewed_disposition"] = disposition
        if disposition["test_evidence"]:
            row["reviewed_test_evidence"] = disposition["test_evidence"]

    unmatched = sorted(set(dispositions) - matched)
    if unmatched:
        raise ValueError(
            f"unmatched {section} disposition keys: {', '.join(unmatched)}"
        )
    return {
        "provided": len(dispositions),
        "matched": len(matched),
        "handled": sum(
            row["state"] in HANDLED_STATES and java_key(row["java"]) in matched
            for row in rows
        ),
    }


def camel_to_snake(name: str) -> str:
    """Convert acronym-bearing Java names such as QLParser to ql_parser."""
    step_one = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return (
        re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", step_one)
        .replace("-", "_")
        .lower()
    )


def java_type_markers(java_type: Node) -> set[str]:
    """返回 Rust 注释中常见的 Java 类型来源写法。"""
    qualified = java_type.qualified_name.replace("::", ".")
    owner = Path(java_type.file_path).stem
    markers = {qualified, java_type.name}
    if java_type.name != owner:
        markers.update(
            {
                f"{owner}.{java_type.name}",
                f"{owner}::{java_type.name}",
                f"{owner} 内部类 {java_type.name}",
            }
        )
    return markers


def doc_mentions_java_type(doc: str, java_type: Node) -> bool:
    if not doc:
        return False
    return any(marker in doc for marker in java_type_markers(java_type))


def doc_mentions_java_method(
    doc: str,
    java_method: Node,
    java_owner_name: str | None,
) -> bool:
    """判断 Rust 文档是否显式追溯到指定 Java 方法。"""
    if not doc:
        return False
    method = java_method.name
    owners = {
        value
        for value in (
            java_owner_name,
            Path(java_method.file_path).stem,
        )
        if value
    }
    owner_patterns = [
        rf"{re.escape(owner)}(?:#|\.){re.escape(method)}(?![A-Za-z0-9_])"
        for owner in owners
    ]
    patterns = [
        rf"#{re.escape(method)}(?![A-Za-z0-9_])",
        rf"(?<![A-Za-z0-9_]){re.escape(method)}\s*\(",
        *owner_patterns,
    ]
    return any(re.search(pattern, doc) for pattern in patterns)


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


def cfg_test_cutoffs(
    rust_root: Path, rust_source_root: Path,
) -> tuple[dict[str, int], dict[str, int]]:
    """Return ``(cutoffs, line_counts)`` for each Rust source file.

    ``cutoffs`` maps file → first ``#[cfg(test)]`` line (or absent if none).
    ``line_counts`` maps file → physical line count in the actual source file.

    When CodeGraph resolves ``include!`` directives it may attribute methods
    to the *hosting* file at line numbers **beyond** the physical line count.
    ``line_counts`` lets callers distinguish such ``include!``-resolved nodes
    from nodes that genuinely reside in a ``#[cfg(test)]`` section.
    """
    cutoffs: dict[str, int] = {}
    line_counts: dict[str, int] = {}
    for path in (rust_root / rust_source_root).rglob("*.rs"):
        relative = path.relative_to(rust_root).as_posix()
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        line_counts[relative] = len(lines)
        for line_number, line in enumerate(lines, 1):
            if re.match(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", line):
                cutoffs[relative] = line_number
                break
    return cutoffs, line_counts


def source_rust_types(
    rust_root: Path,
    rust_source_root: Path,
    test_cutoffs: dict[str, int],
) -> list[Node]:
    """补齐 CodeGraph 当前未提取的 Rust 单元结构体等类型声明。

    CodeGraph 1.4.1 能提取普通结构体、枚举和 trait，但会漏掉
    ``pub struct Foo;`` 形式的单元结构体。源码扫描只作为候选发现兜底，
    不据此推断语义已完成。
    """
    declaration = re.compile(
        r"^\s*(?P<visibility>pub(?:\s*\([^)]*\))?\s+)?"
        r"(?:(?:unsafe|auto)\s+)?"
        r"(?P<kind>struct|enum|trait|type)\s+"
        r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b"
    )
    kind_map = {"type": "type_alias"}
    result: list[Node] = []
    for path in sorted((rust_root / rust_source_root).rglob("*.rs")):
        relative = path.relative_to(rust_root).as_posix()
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        cutoff = test_cutoffs.get(relative, len(lines) + 1)
        for index, line in enumerate(lines[: cutoff - 1], 1):
            match = declaration.match(line)
            if not match:
                continue
            doc_lines: list[str] = []
            cursor = index - 2
            while cursor >= 0:
                stripped = lines[cursor].strip()
                if stripped.startswith("///"):
                    doc_lines.append(stripped[3:].lstrip())
                    cursor -= 1
                    continue
                if stripped.startswith("#[") or not stripped:
                    cursor -= 1
                    continue
                break
            doc_lines.reverse()
            kind = kind_map.get(match.group("kind"), match.group("kind"))
            name = match.group("name")
            result.append(
                Node(
                    id=f"source:{kind}:{relative}:{index}:{name}",
                    kind=kind,
                    name=name,
                    qualified_name=name,
                    file_path=relative,
                    start_line=index,
                    end_line=index,
                    docstring="\n".join(doc_lines),
                    signature=line.strip(),
                    visibility=(
                        "public" if match.group("visibility") else "private"
                    ),
                )
            )
    return result


def source_rust_fields(
    rust_root: Path,
    rust_source_root: Path,
    test_cutoffs: dict[str, int],
) -> list[Node]:
    """发现 CodeGraph 1.4.1 尚未暴露的 Rust 结构体字段。

    字段候选用于识别 JavaBean getter/setter 以及生成式 Parser context
    accessor 的 Rust 数据形态。它只证明“存在可能承接该值的字段”，不证明
    可见性、复制/借用、校验、错误或副作用契约等价。
    """
    struct_start = re.compile(
        r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?"
        r"struct\s+([A-Za-z_][A-Za-z0-9_]*)\b[^;]*\{"
    )
    field = re.compile(
        r"^\s*(?P<visibility>pub(?:\s*\([^)]*\))?\s+)?"
        r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?P<type>.+?)(?:,\s*)?$"
    )
    result: list[Node] = []
    for path in sorted((rust_root / rust_source_root).rglob("*.rs")):
        relative = path.relative_to(rust_root).as_posix()
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        cutoff = test_cutoffs.get(relative, len(lines) + 1)
        depth = 0
        owner: tuple[str, int] | None = None
        for index, line in enumerate(lines[: cutoff - 1], 1):
            if owner and depth <= owner[1]:
                owner = None
            start = struct_start.match(line)
            if start:
                owner = (start.group(1), depth)
            elif owner and depth == owner[1] + 1:
                match = field.match(line)
                if match:
                    doc_lines: list[str] = []
                    cursor = index - 2
                    while cursor >= 0:
                        stripped = lines[cursor].strip()
                        if stripped.startswith("///"):
                            doc_lines.append(stripped[3:].lstrip())
                            cursor -= 1
                            continue
                        if stripped.startswith("#[") or not stripped:
                            cursor -= 1
                            continue
                        break
                    doc_lines.reverse()
                    name = match.group("name")
                    result.append(
                        Node(
                            id=f"source:field:{relative}:{index}:{owner[0]}:{name}",
                            kind="field",
                            name=name,
                            qualified_name=f"{owner[0]}::{name}",
                            file_path=relative,
                            start_line=index,
                            end_line=index,
                            docstring="\n".join(doc_lines),
                            signature=line.strip(),
                            visibility=(
                                "public"
                                if match.group("visibility")
                                else "private"
                            ),
                        )
                    )
            depth += line.count("{") - line.count("}")
    return result


def source_rust_methods(
    rust_root: Path,
    rust_source_root: Path,
    test_cutoffs: dict[str, int],
) -> list[Node]:
    """从源码补齐普通 Rust 方法的完整 rustdoc。

    CodeGraph 1.4.1 对带属性、trait impl 和部分普通方法的 docstring 可能为
    空；源码候选用于恢复显式 Java 方法来源锚点。合并时仍以 CodeGraph 的
    符号身份与调用边为主。
    """
    owner_start = re.compile(
        r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?"
        r"(?:unsafe\s+)?(?:trait|impl(?:\s*<[^>]*>)?)\b"
    )
    function = re.compile(
        r"^\s*(?P<visibility>pub(?:\s*\([^)]*\))?\s+)?"
        r"(?:(?:async|const|unsafe|extern(?:\s+\"[^\"]+\")?)\s+)*"
        r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b"
    )
    result: list[Node] = []
    for path in sorted((rust_root / rust_source_root).rglob("*.rs")):
        relative = path.relative_to(rust_root).as_posix()
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        cutoff = test_cutoffs.get(relative, len(lines) + 1)
        depth = 0
        owner_stack: list[tuple[str, int]] = []
        for index, line in enumerate(lines[: cutoff - 1], 1):
            while owner_stack and depth <= owner_stack[-1][1]:
                owner_stack.pop()
            if owner_start.match(line) and "{" in line:
                owner_match = re.search(
                    r"\bfor\s+([A-Za-z_][A-Za-z0-9_]*)",
                    line,
                )
                if not owner_match:
                    owner_match = re.search(
                        r"\b(?:trait|impl(?:\s*<[^>]*>)?)\s+"
                        r"([A-Za-z_][A-Za-z0-9_]*)",
                        line,
                    )
                if owner_match:
                    owner_stack.append((owner_match.group(1), depth))
            match = function.match(line)
            if match:
                doc_lines: list[str] = []
                cursor = index - 2
                while cursor >= 0:
                    stripped = lines[cursor].strip()
                    if stripped.startswith("///"):
                        doc_lines.append(stripped[3:].lstrip())
                        cursor -= 1
                        continue
                    if stripped.startswith("#[") or not stripped:
                        cursor -= 1
                        continue
                    break
                doc_lines.reverse()
                name = match.group("name")
                owner = owner_stack[-1][0] if owner_stack else path.stem
                result.append(
                    Node(
                        id=f"source:method:{relative}:{index}:{owner}:{name}",
                        kind="method",
                        name=name,
                        qualified_name=f"{owner}::{name}",
                        file_path=relative,
                        start_line=index,
                        end_line=index,
                        docstring="\n".join(doc_lines),
                        signature=line.strip(),
                        visibility=(
                            "public"
                            if match.group("visibility")
                            else "private"
                        ),
                    )
                )
            depth += line.count("{") - line.count("}")
    return result


def source_rust_modules(
    rust_root: Path,
    rust_source_root: Path,
) -> dict[str, Node]:
    """将文件级 ``//!`` 文档暴露为静态工具类/接口的模块候选。"""
    result: dict[str, Node] = {}
    for path in sorted((rust_root / rust_source_root).rglob("*.rs")):
        relative = path.relative_to(rust_root).as_posix()
        docs: list[str] = []
        for line in path.read_text(
            encoding="utf-8",
            errors="replace",
        ).splitlines():
            stripped = line.strip()
            if stripped.startswith("//!"):
                docs.append(stripped[3:].lstrip())
                continue
            if not stripped and docs:
                continue
            break
        if docs:
            result[relative] = Node(
                id=f"source:module:{relative}",
                kind="module",
                name=path.stem,
                qualified_name=relative,
                file_path=relative,
                start_line=1,
                end_line=1,
                docstring="\n".join(docs),
                signature=f"mod {path.stem}",
                visibility="public",
            )
    return result


def merge_rust_types(codegraph_types: list[Node], source_types: list[Node]) -> list[Node]:
    """合并 CodeGraph 类型与源码兜底类型，避免重复候选。"""
    source_by_key = {
        (node.kind, node.name, node.file_path): node
        for node in source_types
    }
    result = []
    for node in codegraph_types:
        source = source_by_key.get((node.kind, node.name, node.file_path))
        result.append(
            replace(node, docstring=source.docstring)
            if source and len(source.docstring) > len(node.docstring)
            else node
        )
    known = {
        (node.kind, node.name, node.file_path)
        for node in codegraph_types
    }
    for node in source_types:
        key = (node.kind, node.name, node.file_path)
        if key not in known:
            result.append(node)
            known.add(key)
    return result


def source_rust_macro_methods(
    rust_root: Path,
    rust_source_root: Path,
    test_cutoffs: dict[str, int],
) -> list[Node]:
    """发现类型内部 ``*_methods! { method(Type); }`` 生成的方法。

    这类方法在编译后真实存在，但 CodeGraph 只看到宏调用，未展开成方法节点。
    """
    type_start = re.compile(
        r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?"
        r"(?:struct|enum|trait)\s+([A-Za-z_][A-Za-z0-9_]*)\b[^;]*\{"
    )
    macro_start = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*_methods!\s*\{")
    macro_entry = re.compile(
        r"^\s*([a-z_][A-Za-z0-9_]*)\s*"
        r"\(\s*[A-Za-z_][A-Za-z0-9_:<>, ]*\s*\)\s*;\s*$"
    )
    result: list[Node] = []
    for path in sorted((rust_root / rust_source_root).rglob("*.rs")):
        relative = path.relative_to(rust_root).as_posix()
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        cutoff = test_cutoffs.get(relative, len(lines) + 1)
        depth = 0
        type_stack: list[tuple[str, int]] = []
        macro_stack: list[tuple[str, int]] = []
        for index, line in enumerate(lines[: cutoff - 1], 1):
            while type_stack and depth <= type_stack[-1][1]:
                type_stack.pop()
            while macro_stack and depth <= macro_stack[-1][1]:
                macro_stack.pop()
            type_match = type_start.match(line)
            if type_match:
                type_stack.append((type_match.group(1), depth))
            if macro_start.search(line) and type_stack:
                macro_stack.append((type_stack[-1][0], depth))
            entry = macro_entry.match(line)
            if entry and macro_stack:
                owner = macro_stack[-1][0]
                name = entry.group(1)
                result.append(
                    Node(
                        id=f"source:macro-method:{relative}:{index}:{owner}:{name}",
                        kind="method",
                        name=name,
                        qualified_name=f"{owner}::{name}",
                        file_path=relative,
                        start_line=index,
                        end_line=index,
                        docstring=(
                            "宏展开方法；候选来源由迁移清单生成器记录，"
                            "仍需语义与测试证据。"
                        ),
                        signature=line.strip(),
                        visibility="public",
                    )
                )
            depth += line.count("{") - line.count("}")
    return result


def merge_rust_methods(
    codegraph_methods: list[Node],
    source_methods: list[Node],
) -> list[Node]:
    """合并 CodeGraph 方法与宏展开方法候选。"""
    source_by_location = {
        (node.name, node.file_path, node.start_line): node
        for node in source_methods
    }
    result = []
    for node in codegraph_methods:
        source = source_by_location.get(
            (node.name, node.file_path, node.start_line)
        )
        result.append(
            replace(node, docstring=source.docstring)
            if source and len(source.docstring) > len(node.docstring)
            else node
        )
    known = {
        (node.name, node.file_path, node.start_line)
        for node in codegraph_methods
    }
    for node in source_methods:
        key = (node.name, node.file_path, node.start_line)
        if key not in known:
            result.append(node)
            known.add(key)
    return result


def is_rust_production(
    node: Node,
    rust_source_root: Path,
    test_cutoffs: dict[str, int],
    file_line_counts: dict[str, int] | None = None,
) -> bool:
    """Determine whether a Rust CodeGraph node is production (non-test) code.

    When ``file_line_counts`` is provided, nodes whose ``start_line`` exceeds
    the physical file length are accepted regardless of the ``#[cfg(test)]``
    cutoff.  CodeGraph resolves ``include!`` directives and may attribute
    included-file methods to the hosting file at virtual line numbers beyond
    the physical file boundary — those methods are production code, not tests.
    """
    prefix = rust_source_root.as_posix().rstrip("/") + "/"
    if not (node.file_path.startswith(prefix) and node.file_path.endswith(".rs")):
        return False
    if file_line_counts is not None:
        physical = file_line_counts.get(node.file_path)
        if physical is not None and node.start_line > physical:
            # include!-resolved node — always production
            return True
    return node.start_line < test_cutoffs.get(node.file_path, 2**31)


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
                nearby = "\n".join(lines[max(0, index - 4) : index + 36])
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
                        "declared_adaptation": "ADAPTED" in nearby,
                    }
                )
    return dict(markers)


def method_candidates(
    java_method: Node,
    rust_members: list[Node],
    candidate_files: set[str],
    owner_names: set[str],
    java_owner_name: str | None,
    java_owner_qualified_name: str | None = None,
) -> list[Node]:
    expected_name = camel_to_snake(java_method.name)
    names = {expected_name}
    special_names: set[str] = set()
    for prefix in ("get", "set", "is"):
        if java_method.name.startswith(prefix) and len(java_method.name) > len(prefix):
            suffix = java_method.name[len(prefix) :]
            if suffix[0].isupper():
                adapted = camel_to_snake(suffix)
                names.add(adapted)
                if prefix == "set":
                    names.add(f"set_{adapted}")
    if java_method.name == "toString":
        names.add("to_string")
    # Merged visitor methods: Java has separate visitOptionalMethodInvoke,
    # visitSpreadMethodInvoke, visitOptionalFieldAccess, visitSpreadFieldAccess
    # but Rust merges them into visit_method_invoke / visit_field_access with ChainKind.
    _visitor_merge_map = {
        "visitOptionalMethodInvoke": "visit_method_invoke",
        "visitSpreadMethodInvoke": "visit_method_invoke",
        "visitOptionalFieldAccess": "visit_field_access",
        "visitSpreadFieldAccess": "visit_field_access",
    }
    # Pass-through visitor methods: Java calls super.visitXxx(ctx).
    # In Rust these are generated by the base visitor macro and dispatched
    # by the parse tree enum, so they exist as visit_xxx on the base visitor.
    _visitor_passthrough = {
        "visitArrayInitializer",
        "visitConstExpr",
    }
    if java_method.name in _visitor_merge_map:
        names.add(_visitor_merge_map[java_method.name])
    if java_method.name in _visitor_passthrough:
        expected_visitor_name = camel_to_snake(java_method.name)
        names.add(expected_visitor_name)
    if (
        java_method.name == "accept"
        and java_owner_name
        and java_owner_name.endswith("Context")
    ):
        visitor_target = re.sub(r"Context$", "", java_owner_name)
        special_names.update(
            {
                "accept",
                f"visit_{camel_to_snake(visitor_target)}",
            }
        )
    # Known method renames where Rust uses a different name
    _method_renames = {
        ("SyntaxTreeFactory", "buildTree"): {"build_tree", "build_tree_from_tokens"},
        ("QLambdaInvocationHandler", "invoke"): {"invoke_abstract", "invoke_to_string", "create_closure"},
    }
    rename_key = (java_owner_name, java_method.name) if java_owner_name else None
    if rename_key in _method_renames:
        names.update(_method_renames[rename_key])
    if re.fullmatch(r"[A-Z][A-Z0-9_]*", java_method.name):
        token_name = java_method.name.lower()
        names.update({token_name, f"{token_name}_token", "token", "terminal"})
    if java_method.name in {
        Path(java_method.file_path).stem,
        java_owner_name,
    }:
        names.update({"new", "default", "with_init_options"})
        if java_owner_name:
            names.add(java_owner_name)
    parser_generated_owner = (
        java_owner_name is not None
        and java_owner_name.endswith("Context")
        and Path(java_method.file_path).stem == "QLParser"
    )
    number_math_impl = (
        java_owner_name == "NumberMath"
        and java_method.name.endswith("Impl")
    )
    return [
        member
        for member in rust_members
        if member.name in names
        and (
            member.file_path in candidate_files
            or any(
                member.qualified_name.startswith(f"{owner_name}::")
                for owner_name in owner_names
            )
            or (
                number_math_impl
                and "/number/" in member.file_path
            )
            or (
                java_owner_qualified_name is not None
                and java_owner_qualified_name in CROSS_TYPE_DELEGATION
                and any(
                    member.qualified_name.startswith(f"{delegate}::")
                    for delegate in CROSS_TYPE_DELEGATION[java_owner_qualified_name]
                )
            )
        )
        or (
            parser_generated_owner
            and member.kind == "field"
            and "/aparser/" in member.file_path
            and (
                expected_name in member.name
                or member.name in expected_name
                or member.name in names
            )
        )
        or (
            (
                member.file_path in candidate_files
                or any(
                    member.qualified_name.startswith(f"{owner_name}::")
                    for owner_name in owner_names
                )
            )
            and doc_mentions_java_method(
                member.docstring,
                java_method,
                java_owner_name,
            )
        )
        or (
            member.name in special_names
            and "/aparser/" in member.file_path
            and member.kind in {"method", "function"}
        )
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

    java_state = git_state(java_root)
    rust_state = git_state(rust_root)
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
    test_cutoffs, file_line_counts = cfg_test_cutoffs(rust_root, rust_source_root)
    codegraph_rust_types = [
        node
        for node in rust_nodes
        if node.kind in RUST_TYPE_KINDS
        and is_rust_production(node, rust_source_root, test_cutoffs, file_line_counts)
    ]
    rust_types = merge_rust_types(
        codegraph_rust_types,
        source_rust_types(rust_root, rust_source_root, test_cutoffs),
    )
    rust_modules_by_file = source_rust_modules(rust_root, rust_source_root)
    rust_methods = merge_rust_methods(
        [
            node
            for node in rust_nodes
            if node.kind in {"method", "function"}
            and is_rust_production(node, rust_source_root, test_cutoffs, file_line_counts)
        ],
        [
            *source_rust_methods(
                rust_root,
                rust_source_root,
                test_cutoffs,
            ),
            *source_rust_macro_methods(
                rust_root,
                rust_source_root,
                test_cutoffs,
            ),
        ],
    )
    rust_fields = source_rust_fields(
        rust_root,
        rust_source_root,
        test_cutoffs,
    )
    rust_members = [*rust_methods, *rust_fields, *rust_types]

    rust_files = {
        path.relative_to(rust_root).as_posix()
        for path in (rust_root / rust_source_root).rglob("*.rs")
    }
    rust_files_by_name: dict[str, list[str]] = defaultdict(list)
    for path in rust_files:
        rust_files_by_name[Path(path).name].append(path)
    rust_types_by_file: dict[str, list[Node]] = defaultdict(list)
    rust_types_by_name: dict[str, list[Node]] = defaultdict(list)
    for node in rust_types:
        rust_types_by_file[node.file_path].append(node)
        rust_types_by_name[node.name].append(node)

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
    files_for_java_type: dict[str, set[str]] = {}
    rust_names_for_java_type: dict[str, set[str]] = {}
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
            traced_types = [
                node
                for node in rust_types_by_file.get(expected_file, [])
                if node not in exact_types
                and doc_mentions_java_type(node.docstring, java_type)
            ]
            module_candidate = rust_modules_by_file.get(expected_file)
            if (
                module_candidate
                and doc_mentions_java_type(
                    module_candidate.docstring,
                    java_type,
                )
            ):
                traced_types.append(module_candidate)
            candidate_types = [*exact_types, *traced_types]
            state = "UNVERIFIED" if candidate_types else "PARTIAL"
        elif same_name_files:
            candidate_files = set(same_name_files)
            exact_types = [
                node
                for path in same_name_files
                for node in rust_types_by_file.get(path, [])
                if node.name == java_type.name
            ]
            candidate_types = exact_types
            state = "MISPLACED"
        else:
            candidate_files = set()
            exact_types = []
            candidate_types = []
            state = "MISSING"
        assert state in STRICT_STATES
        state_counts[state] += 1
        files_for_java_type[java_type.id] = candidate_files
        rust_names_for_java_type[java_type.id] = {
            node.name for node in candidate_types
        }
        object_rows.append(
            {
                "java": node_dict(java_type),
                "expected_rust_file": expected_file,
                "candidate_rust_files": sorted(candidate_files),
                "candidate_rust_types": [
                    node_dict(node) for node in candidate_types
                ],
                "candidate_mapping": (
                    "EXACT_NAME"
                    if exact_types
                    else "JAVA_SOURCE_TRACE"
                    if candidate_types
                    else None
                ),
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
        exact_types = list(rust_types_by_name.get(nested_type.name, []))
        traced_types = [
            node
            for node in rust_types
            if node not in exact_types
            and doc_mentions_java_type(node.docstring, nested_type)
        ]
        candidate_types = sorted(
            [*exact_types, *traced_types],
            key=lambda node: (node.file_path, node.start_line),
        )
        candidate_files = {node.file_path for node in candidate_types}
        files_for_java_type[nested_type.id] = candidate_files
        rust_names_for_java_type[nested_type.id] = {
            node.name for node in candidate_types
        }
        if candidate_types:
            state = "UNVERIFIED"
        else:
            state = "MISSING"
        nested_state_counts[state] += 1
        nested_rows.append(
            {
                "java": node_dict(nested_type),
                "owning_java_object": parent_row["java"]["qualified_name"],
                "candidate_rust_files": sorted(candidate_files),
                "candidate_rust_types": [
                    node_dict(node) for node in candidate_types
                ],
                "candidate_mapping": (
                    "EXACT_NAME"
                    if exact_types
                    else "JAVA_SOURCE_TRACE"
                    if candidate_types
                    else None
                ),
                "state": state,
                "semantic_evidence": [],
                "review_note": (
                    "内部类型仍需独立核对字段、构造和行为契约。"
                    if candidate_types
                    else "未在 Rust 生产源码中发现同名内部类型。"
                ),
            }
        )

    primary_by_file = {item.file_path: item for item in primary_types}
    method_rows: list[dict[str, Any]] = []
    method_state_counts: dict[str, int] = defaultdict(int)
    for java_method in java_methods:
        owner = nearest_owner(java_method, java_types_by_file)
        primary = primary_by_file.get(java_method.file_path)
        candidate_files = set()
        owner_names: set[str] = set()
        if primary:
            candidate_files.update(files_for_java_type.get(primary.id, set()))
            owner_names.update(rust_names_for_java_type.get(primary.id, set()))
        if owner:
            candidate_files.update(files_for_java_type.get(owner.id, set()))
            owner_names.update(rust_names_for_java_type.get(owner.id, set()))
        candidates = method_candidates(
            java_method,
            rust_members,
            candidate_files,
            owner_names,
            owner.name if owner else None,
            java_owner_qualified_name=owner.qualified_name if owner else None,
        )
        # Auto-mark methods based on their owner type category
        owner_qualified = owner.qualified_name if owner else None
        method_key = f"{owner_qualified}#{java_method.name}" if owner_qualified else None
        _is_parser_context = (
            owner_qualified is not None
            and owner_qualified.endswith("Context")
            and Path(java_method.file_path).stem == "QLParser"
        )
        # ANTLR token/mode names that don't exist in Rust's custom lexer
        _antlr_token_na = {"StaticStringCharacters", "DyStrExprStart", "SelectorVariable_VANME"}
        if not candidates:
            if owner_qualified in ANTLR_FRAMEWORK_TYPES:
                state = "PLATFORM_NA"
            elif method_key in REFLECTION_NA_TYPES:
                state = "PLATFORM_NA"
            elif java_method.name in _antlr_token_na and _is_parser_context:
                state = "PLATFORM_NA"
            elif (
                owner_qualified in DERIVE_HANDLED_TYPES
                and java_method.name in DERIVE_METHOD_NAMES
            ):
                state = "RUST_EXTENSION"
            elif method_key and method_key in KNOWN_RELOCATIONS:
                state = KNOWN_RELOCATIONS[method_key]
            elif (
                owner_qualified is not None
                and "ReflectLoader::" in owner_qualified
                and java_method.name in REFLECT_LOADER_INNER_GETTERS
            ):
                state = "RUST_EXTENSION"
            else:
                state = "MISSING"
        else:
            state = "UNVERIFIED"
        method_state_counts[state] += 1
        source_class = Path(java_method.file_path).stem
        marker_key = f"{source_class}#{java_method.name}"
        method_rows.append(
            {
                "java": node_dict(java_method),
                "java_owner": node_dict(owner) if owner else None,
                "direct_java_calls": java_call_graph.get(java_method.id, []),
                "candidate_rust_members": [
                    node_dict(candidate) for candidate in candidates
                ],
                "candidate_rust_methods": [
                    node_dict(candidate)
                    for candidate in candidates
                    if candidate.kind in {"method", "function"}
                ],
                "candidate_mapping_kinds": sorted(
                    {candidate.kind for candidate in candidates}
                ),
                "test_evidence": test_markers.get(marker_key, []),
                "state": state,
                "semantic_evidence": [],
                "review_note": (
                    "同名候选不证明重载、参数、返回值、错误和副作用等价。"
                    if state == "UNVERIFIED"
                    else "未发现方法、字段、类型或分派级 Rust 候选；需要人工确认适配名或补齐实现。"
                ),
            }
        )

    disposition_stats: dict[str, dict[str, int]] = {}
    current_rust_source_fingerprint = source_tree_fingerprint(
        rust_root,
        rust_source_root,
    )
    if args.dispositions:
        dispositions = load_dispositions(
            args.dispositions.resolve(),
            java_sha=java_state["sha"],
            rust_sha=rust_state["sha"],
            rust_source_fingerprint=current_rust_source_fingerprint,
            rust_root=rust_root,
        )
        disposition_stats = {
            "objects": apply_dispositions(
                object_rows,
                dispositions["objects"],
                section="objects",
            ),
            "nested_java_types": apply_dispositions(
                nested_rows,
                dispositions["nested_java_types"],
                section="nested_java_types",
            ),
            "methods": apply_dispositions(
                method_rows,
                dispositions["methods"],
                section="methods",
            ),
        }
        state_counts = count_states(object_rows)
        nested_state_counts = count_states(nested_rows)
        method_state_counts = count_states(method_rows)
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
    member_candidates = [
        candidate
        for row in method_rows
        for candidate in row["candidate_rust_members"]
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
        "java_methods_with_member_candidate": sum(
            bool(row["candidate_rust_members"]) for row in method_rows
        ),
        "java_methods_with_method_candidate": sum(
            bool(row["candidate_rust_methods"]) for row in method_rows
        ),
        "rust_member_candidates": len(member_candidates),
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
            "reviewed_disposition_rule": (
                "handled states require an exact baseline, exact Java key, "
                "semantic rationale, current Rust source anchor, and current "
                "semantic test anchor"
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
                "source_fingerprint": current_rust_source_fingerprint,
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
            "reviewed_dispositions": disposition_stats,
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
        java_test_rows = []
        java_test_mapping_counts: dict[str, int] = defaultdict(int)
        for java_test in test_inventory["java_tests"]:
            marker_key = (
                f"{Path(java_test['file']).stem}#{java_test['name']}"
            )
            evidence = test_markers.get(marker_key, [])
            discovery_state = "MAPPED" if evidence else "MISSING"
            java_test_mapping_counts[discovery_state] += 1
            java_test_rows.append(
                {
                    "java": java_test,
                    "marker": marker_key,
                    "rust_test_evidence": evidence,
                    "discovery_state": discovery_state,
                    "parity_state": "UNVERIFIED",
                    "review_note": (
                        "显式来源标记仅证明测试映射存在；仍需核对输入、断言、"
                        "异常、边界和副作用后标记 EXACT 或 ADAPTED。"
                        if evidence
                        else "未发现显式 Rust 测试来源标记。"
                    ),
                }
            )
        manifest["summary"]["java_test_mapping_states"] = dict(
            sorted(java_test_mapping_counts.items())
        )
        manifest["java_test_mappings"] = java_test_rows
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
                "dispositions": (
                    str(args.dispositions) if args.dispositions else None
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
