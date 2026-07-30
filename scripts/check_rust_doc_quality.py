#!/usr/bin/env python3
"""检查 Rust 公共 API 的中文语义、迁移来源和重要入口文档章节。"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "crates"

FORBIDDEN_FRAGMENTS = (
    "结构体的 Rust 实现，保留对应对象的领域职责与公开契约",
    "对应的领域职责",
)

LINT_ROOTS = (
    "crates/qlexpress/src/lib.rs",
    "crates/qlexpress-derive/src/lib.rs",
    "crates/qlexpress-process/src/lib.rs",
    "crates/qlexpress-process/src/main.rs",
    "crates/qlexpress-verification/src/main.rs",
)

SOURCE_MARKER = re.compile(
    r"对应(?:或承接)? Java|Java 无|Rust (?:新增|适配|安全增强|原生适配)"
)
PUBLIC_OBJECT = re.compile(
    r"^\s*pub\s+(?:unsafe\s+)?(?:struct|enum|trait|type)\s+[A-Za-z_]"
)
PUBLIC_FUNCTION = re.compile(
    r"^\s*pub\s+(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)
CHINESE = re.compile(r"[\u3400-\u9fff]")


@dataclass(frozen=True)
class ImportantApi:
    path: str
    name: str
    sections: tuple[str, ...]
    signature_contains: str | None = None


IMPORTANT_APIS = (
    ImportantApi("crates/qlexpress/src/express4_runner.rs", "new", ("# Returns",)),
    ImportantApi(
        "crates/qlexpress/src/express4_runner.rs",
        "with_init_options",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress/src/express4_runner.rs",
        "execute",
        ("# Arguments", "# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress/src/express4_runner.rs",
        "execute_template",
        ("# Arguments", "# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress/src/express4_runner.rs",
        "execute_with_context",
        ("# Arguments", "# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress/src/express4_runner.rs",
        "execute_checked",
        ("# Arguments", "# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress/src/express4_runner.rs",
        "execute_checked_with_context",
        ("# Arguments", "# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress/src/init_options.rs",
        "selector_start",
        ("# Arguments", "# Returns", "# Panics"),
        "mut self",
    ),
    ImportantApi(
        "crates/qlexpress/src/init_options.rs",
        "selector_end",
        ("# Arguments", "# Returns", "# Panics"),
        "mut self",
    ),
    ImportantApi(
        "crates/qlexpress/src/security/sandbox_profile.rs",
        "secure",
        ("# Returns",),
    ),
    ImportantApi(
        "crates/qlexpress/src/security/sandbox_profile.rs",
        "validate",
        ("# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress/src/security/resource_limits.rs",
        "validate",
        ("# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress/src/security/capability_policy.rs",
        "allow_only",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress/src/security/capability_policy.rs",
        "allow",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress/src/security/capability_policy.rs",
        "is_allowed",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress/src/security/capability_policy.rs",
        "is_method_allowed",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress/src/runtime/native_registry.rs",
        "register_type",
        ("# Arguments",),
    ),
    ImportantApi(
        "crates/qlexpress/src/runtime/native_registry.rs",
        "register_method",
        ("# Arguments",),
    ),
    ImportantApi(
        "crates/qlexpress/src/runtime/native_registry.rs",
        "load_constructor_for_args",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress/src/runtime/native_registry.rs",
        "load_field_with_security",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress/src/runtime/native_registry.rs",
        "resolve_method_for_args",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress-process/src/process_worker.rs",
        "new",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress-process/src/process_worker.rs",
        "execute",
        ("# Arguments", "# Returns", "# Errors"),
    ),
    ImportantApi(
        "crates/qlexpress-process/src/worker_response.rs",
        "success",
        ("# Arguments", "# Returns"),
    ),
    ImportantApi(
        "crates/qlexpress-process/src/worker_response.rs",
        "failure",
        ("# Arguments", "# Returns"),
    ),
)


def rust_sources() -> list[Path]:
    return sorted(CRATES.glob("*/src/**/*.rs"))


def preceding_docs(lines: list[str], index: int) -> str:
    cursor = index - 1
    collected: list[str] = []
    while cursor >= 0:
        stripped = lines[cursor].lstrip()
        if stripped.startswith("///"):
            collected.append(stripped[3:].strip())
            cursor -= 1
            continue
        if stripped.startswith("#[") or not stripped.strip():
            cursor -= 1
            continue
        break
    return "\n".join(reversed(collected))


def fail(errors: list[str], path: Path, line: int, message: str) -> None:
    errors.append(f"{path.relative_to(ROOT)}:{line}: {message}")


def check_forbidden(errors: list[str], sources: list[Path]) -> None:
    for path in sources:
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for fragment in FORBIDDEN_FRAGMENTS:
                if fragment in line:
                    fail(errors, path, line_no, f"禁止模板化注释：{fragment}")


def check_lints(errors: list[str]) -> None:
    marker = "#![warn(missing_docs)]"
    for relative in LINT_ROOTS:
        path = ROOT / relative
        if marker not in path.read_text(encoding="utf-8"):
            fail(errors, path, 1, f"crate 根缺少 {marker}")


def check_public_objects(errors: list[str], sources: list[Path]) -> int:
    count = 0
    for path in sources:
        lines = path.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines):
            if not PUBLIC_OBJECT.match(line):
                continue
            count += 1
            docs = preceding_docs(lines, index)
            if not docs:
                fail(errors, path, index + 1, "公开对象缺少 Rustdoc")
                continue
            if not CHINESE.search(docs):
                fail(errors, path, index + 1, "公开对象 Rustdoc 缺少中文用途说明")
            if "crates/qlexpress/src/" in path.as_posix() and not SOURCE_MARKER.search(docs):
                fail(errors, path, index + 1, "迁移对象缺少 Java 来源或 Rust 适配说明")
    return count


def find_function_docs(
    path: Path, name: str, signature_contains: str | None
) -> tuple[int, str] | None:
    lines = path.read_text(encoding="utf-8").splitlines()
    for index, line in enumerate(lines):
        match = PUBLIC_FUNCTION.match(line)
        if (
            match
            and match.group(1) == name
            and (signature_contains is None or signature_contains in line)
        ):
            return index + 1, preceding_docs(lines, index)
    return None


def check_important_apis(errors: list[str]) -> int:
    for api in IMPORTANT_APIS:
        path = ROOT / api.path
        located = find_function_docs(path, api.name, api.signature_contains)
        if located is None:
            fail(errors, path, 1, f"找不到重要公共 API：{api.name}")
            continue
        line_no, docs = located
        if not CHINESE.search(docs):
            fail(errors, path, line_no, f"{api.name} 缺少中文语义说明")
        for section in api.sections:
            if section not in docs:
                fail(errors, path, line_no, f"{api.name} 缺少 {section} 章节")
    return len(IMPORTANT_APIS)


def main() -> int:
    errors: list[str] = []
    sources = rust_sources()
    check_forbidden(errors, sources)
    check_lints(errors)
    public_objects = check_public_objects(errors, sources)
    important_apis = check_important_apis(errors)

    if errors:
        print("Rust 文档质量检查失败：", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(
        "Rust 文档质量检查通过："
        f"{len(sources)} 个源码文件，{public_objects} 个公开对象，"
        f"{important_apis} 个重要公共 API。"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
