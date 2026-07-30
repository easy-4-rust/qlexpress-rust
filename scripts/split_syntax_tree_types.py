#!/usr/bin/env python3
"""Split generated QLParser AST declarations into one Rust file per type.

The behavioral impl blocks intentionally stay in ``syntax_tree_factory.rs``;
Rust permits inherent/trait impls outside the declaration module.  This keeps
the migration mechanical while satisfying the one-object-per-file invariant.
"""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
APARSER = ROOT / "crates/qlexpress/src/aparser"
SOURCE = APARSER / "syntax_tree_factory.rs"
MOD = APARSER / "mod.rs"
TYPE = re.compile(r"^pub (?:struct|enum|trait|union) ([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)


def snake(name: str) -> str:
    first = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", first).lower()


def declaration_end(text: str, start: int) -> int:
    brace = text.find("{", start)
    if brace < 0:
        semicolon = text.find(";", start)
        if semicolon < 0:
            raise ValueError(f"unterminated declaration at {start}")
        return semicolon + 1
    depth = 0
    for offset in range(brace, len(text)):
        char = text[offset]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return offset + 1
    raise ValueError(f"unterminated declaration at {start}")


def doc_start(text: str, start: int) -> int:
    line_start = text.rfind("\n", 0, start) + 1
    lines = text[:line_start].splitlines(keepends=True)
    index = len(lines)
    while index:
        stripped = lines[index - 1].strip()
        if stripped.startswith(("///", "#[")) or not stripped:
            index -= 1
            continue
        break
    return sum(len(line) for line in lines[:index])


def imports_for(name: str, body: str, all_names: set[str]) -> list[str]:
    imports = []
    for dependency in sorted(all_names):
        if dependency == name:
            continue
        if re.search(rf"\b{re.escape(dependency)}\b", body):
            imports.append(f"use super::{snake(dependency)}::{dependency};")
    if re.search(r"\bTerminalNode\b", body):
        imports.append("use super::terminal_node::TerminalNode;")
    if re.search(r"\bToken\b", body):
        imports.append("use super::token::Token;")
    return imports


def main() -> int:
    text = SOURCE.read_text(encoding="utf-8")
    matches = list(TYPE.finditer(text))
    if not matches:
        print("syntax tree declarations already split")
        return 0
    names = {match.group(1) for match in matches}
    spans: list[tuple[int, int, str, str]] = []
    for match in matches:
        name = match.group(1)
        start = doc_start(text, match.start())
        end = declaration_end(text, match.start())
        body = text[start:end].strip() + "\n"
        spans.append((start, end, name, body))

    for _, _, name, body in spans:
        imports = imports_for(name, body, names)
        header = (
            "//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。\n"
        )
        if imports:
            header += "\n" + "\n".join(imports) + "\n"
        path = APARSER / f"{snake(name)}.rs"
        if path.exists():
            if path.read_text(encoding="utf-8") != header + "\n" + body:
                raise FileExistsError(path)
        else:
            path.write_text(header + "\n" + body, encoding="utf-8")

    for start, end, _, _ in reversed(spans):
        text = text[:start] + text[end:]

    reexports = "\n".join(
        f"pub use super::{snake(name)}::{name};" for _, _, name, _ in spans
    )
    marker = "use super::terminal_node::TerminalNode;\n"
    if marker not in text:
        raise ValueError("syntax_tree_factory import marker not found")
    replacement = (
        marker
        + "\n"
        + reexports
        + "\n\n"
        + "/// 创建和承载 QLParser 语法树对象的工厂身份。\n"
        + "///\n"
        + "/// 对应 Java: `com.alibaba.qlexpress4.aparser.SyntaxTreeFactory`。\n"
        + "pub struct SyntaxTreeFactory;\n"
    )
    SOURCE.write_text(text.replace(marker, replacement, 1), encoding="utf-8")

    mod_text = MOD.read_text(encoding="utf-8")
    mod_marker = "pub mod trace_expression_visitor;\n"
    if mod_marker not in mod_text:
        raise ValueError("aparser mod marker not found")
    declarations = "\n".join(
        f"/// Java QLParser 内部 AST 类型 `{name}`。\npub mod {snake(name)};"
        for _, _, name, _ in spans
    )
    MOD.write_text(
        mod_text.replace(mod_marker, mod_marker + declarations + "\n", 1),
        encoding="utf-8",
    )
    print(f"split {len(spans)} syntax tree types")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
