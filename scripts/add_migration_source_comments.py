#!/usr/bin/env python3
"""为 Rust 公开迁移项补充可审计的 Java 来源行。

该工具只添加来源信息，不生成职责说明，也不会覆盖已有 rustdoc。职责、参数、
返回值和异常语义仍应由源文件中的人工可读中文 rustdoc 描述。
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path


PUBLIC_ITEM = re.compile(
    r"^(?P<indent>[ \t]*)pub(?:\([^)]*\))?\s+(?:async\s+)?"
    r"(?:unsafe\s+)?(?P<kind>fn|struct|enum|trait|union)\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    re.MULTILINE,
)
PACKAGE = re.compile(r"^\s*package\s+([A-Za-z0-9_.]+)\s*;", re.MULTILINE)
SKIP_PARTS = {".git", "target", "vendor", "generated", "tests", "examples", "benches"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-root", type=Path, required=True)
    parser.add_argument("--java-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def normalized(name: str) -> str:
    return re.sub(r"[^a-z0-9]", "", name.lower())


def snake_to_lower_camel(name: str) -> str:
    head, *tail = name.split("_")
    return head + "".join(part[:1].upper() + part[1:] for part in tail)


def java_classes(java_root: Path) -> dict[str, list[str]]:
    classes: dict[str, list[str]] = {}
    for path in java_root.rglob("*.java"):
        text = path.read_text(encoding="utf-8", errors="replace")
        package_match = PACKAGE.search(text)
        if not package_match:
            continue
        fqcn = f"{package_match.group(1)}.{path.stem}"
        classes.setdefault(normalized(path.stem), []).append(fqcn)
    return classes


def source_for(
    path: Path,
    item_name: str,
    item_kind: str,
    classes: dict[str, list[str]],
) -> str:
    candidates = classes.get(normalized(path.stem), [])
    if item_kind != "fn":
        candidates = classes.get(normalized(item_name), candidates)
    if len(candidates) == 1:
        source = candidates[0]
        if item_kind == "fn":
            source += f"#{snake_to_lower_camel(item_name)}"
        return source
    return "无（Rust 原生适配）"


def preceding_comment(text: str, offset: int, lines: int = 10) -> str:
    return "\n".join(text[:offset].splitlines()[-lines:])


def rust_files(root: Path) -> list[Path]:
    result = []
    for path in root.rglob("*.rs"):
        relative = path.relative_to(root)
        if any(part in SKIP_PARTS for part in relative.parts):
            continue
        if path.name == "tests.rs" or path.stem.endswith(("_test", "_tests")):
            continue
        if "src" in relative.parts:
            result.append(path)
    return sorted(result)


def update_file(path: Path, classes: dict[str, list[str]], check: bool) -> int:
    text = path.read_text(encoding="utf-8")
    production_end = text.find("#[cfg(test)]")
    production_text = text if production_end < 0 else text[:production_end]
    insertions: list[tuple[int, str]] = []
    for match in PUBLIC_ITEM.finditer(production_text):
        if "对应 Java" in preceding_comment(text, match.start()):
            continue
        source = source_for(path, match.group("name"), match.group("kind"), classes)
        insertions.append(
            (
                match.start(),
                f"{match.group('indent')}///\n"
                f"{match.group('indent')}/// 对应 Java: {source}。\n",
            )
        )
    if insertions and not check:
        for offset, comment in reversed(insertions):
            text = text[:offset] + comment + text[offset:]
        path.write_text(text, encoding="utf-8")
    return len(insertions)


def main() -> int:
    args = parse_args()
    rust_root = args.rust_root.resolve()
    classes = java_classes(args.java_root.resolve())
    changed_items = 0
    changed_files = 0
    for path in rust_files(rust_root):
        count = update_file(path, classes, args.check)
        if count:
            changed_files += 1
            changed_items += count
    action = "需要补充" if args.check else "已补充"
    print(f"{action}来源注释：{changed_items} 项，{changed_files} 个文件")
    return 1 if args.check and changed_items else 0


if __name__ == "__main__":
    raise SystemExit(main())
