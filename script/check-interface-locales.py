#!/usr/bin/env python3
"""Validate marked interface translations and report remaining display literals."""

import argparse
from collections import Counter
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "crates/warpui_core/src/locales/interface.json"
RAW_STRING = re.compile(r'(?:b|c)?r(#+)?"')
CHARACTER = re.compile(r"'(?:\\(?:u\{[0-9A-Fa-f]+\}|x[0-9A-Fa-f]{2}|.)|[^'\\\n])'")


def decode_rust_string(literal):
    inner = literal[1:-1]
    result = []
    offset = 0
    escapes = {'n': '\n', 'r': '\r', 't': '\t', '0': '\0', '"': '"', "'": "'", '\\': '\\'}
    while offset < len(inner):
        if inner[offset] != '\\':
            result.append(inner[offset])
            offset += 1
            continue
        offset += 1
        character = inner[offset]
        offset += 1
        if character in escapes:
            result.append(escapes[character])
        elif character == 'x':
            result.append(chr(int(inner[offset:offset + 2], 16)))
            offset += 2
        elif character == 'u' and inner[offset] == '{':
            end = inner.index('}', offset)
            result.append(chr(int(inner[offset + 1:end].replace('_', ''), 16)))
            offset = end + 1
        elif character in '\r\n':
            while offset < len(inner) and inner[offset].isspace():
                offset += 1
        else:
            raise ValueError(f"Unsupported Rust string escape: {character!r}")
    return ''.join(result)


def rust_strings(source):
    """Yield string spans while excluding comments, character literals and lifetimes."""
    offset = 0
    while offset < len(source):
        if source.startswith("//", offset):
            newline = source.find("\n", offset)
            offset = len(source) if newline < 0 else newline + 1
            continue
        if source.startswith("/*", offset):
            depth = 1
            offset += 2
            while offset < len(source) and depth:
                if source.startswith("/*", offset):
                    depth += 1
                    offset += 2
                elif source.startswith("*/", offset):
                    depth -= 1
                    offset += 2
                else:
                    offset += 1
            continue
        raw = RAW_STRING.match(source, offset)
        if raw and (offset == 0 or not (source[offset - 1].isalnum() or source[offset - 1] == "_")):
            delimiter = '"' + (raw[1] or "")
            end = source.find(delimiter, raw.end())
            if end < 0:
                raise ValueError("Unterminated Rust raw string")
            end += len(delimiter)
            yield offset, end, source[raw.end():end - len(delimiter)]
            offset = end
            continue
        if source[offset] == "'":
            character = CHARACTER.match(source, offset)
            if character:
                offset = character.end()
                continue
        if source[offset] != '"':
            offset += 1
            continue
        start = offset
        offset += 1
        while offset < len(source):
            if source[offset] == "\\":
                offset += 2
            elif source[offset] == '"':
                offset += 1
                break
            else:
                offset += 1
        literal = source[start:offset]
        try:
            value = decode_rust_string(literal)
        except ValueError as error:
            raise ValueError(f"Rust string at line {source.count(chr(10), 0, start) + 1}: {literal[:80]!r}") from error
        yield start, offset, value


def fields(template):
    result = []
    ordinal = 0
    for match in re.finditer(r"\{\{|\}\}|\{([^{}]*)\}", template):
        if match[0] in ("{{", "}}"):
            continue
        field = match[1]
        if not field or field.startswith(":"):
            field = str(ordinal) + field
            ordinal += 1
        result.append(field)
    return Counter(result)


DISPLAY = re.compile(
    r"(?:Text::new(?:_inline)?|FormattedTextElement::from_str|CustomMenuItem::new"
    r"|Menu::new|DropdownItem::new|Category::new|MenuItemLabelText::new"
    r"|\.(?:with_label|with_tooltip|with_text_label|with_title|with_placeholder"
    r"|set_placeholder|with_secondary_text|with_subtext|span|label|link|paragraph))"
    r"\s*\(\s*$"
)

# Brand/technical names and terminal preview content must retain their spelling.
VERBATIM = {
    "app/src/app_menus.rs": {"Warpai", "AI"},
    "app/src/drive/index.rs": {"Warpai Drive"},
    "app/src/settings_view/about_page.rs": {"Warpai"},
    "app/src/settings_view/ai_page.rs": {"Warpai Agent"},
    "app/src/settings_view/features/external_editor.rs": {"Warpai", "$EDITOR"},
    "app/src/settings_view/mcp_servers/edit_page.rs": {"JSON"},
    "app/src/settings_view/privacy_page.rs": {"ZDR"},
    "app/src/settings_view/warpify_page.rs": {"SSH"},
    "app/src/terminal/view/block_onboarding/onboarding_prompt_block.rs": {"(myenv)", " ~/myproject", " git:(", "main"},
    "app/src/themes/theme.rs": {"ls", "dir   ", "executable   ", "file"},
}
KEY_LABEL_FILES = {
    "app/src/code_review/git_dialog/mod.rs",
    "app/src/settings_view/mcp_servers/installation_modal.rs",
    "app/src/settings_view/mcp_servers/update_modal.rs",
    "app/src/tab_configs/new_worktree_modal.rs",
    "app/src/tab_configs/params_modal.rs",
}


def validate(catalog):
    for source, translations in catalog.items():
        assert isinstance(translations, list) and len(translations) == 2, source
        for translation in translations:
            assert isinstance(translation, str) and translation.strip(), source
            assert fields(source) == fields(translation), (source, translation)
    marked = 0
    remaining = []
    for folder in ("app/src", "crates/warpui_core/src", "crates/ui_components/src"):
        for path in sorted((ROOT / folder).rglob("*.rs")):
            if path.name.endswith(("_test.rs", "_tests.rs")) or path.name == "localization.rs":
                continue
            source = path.read_text()
            relative = str(path.relative_to(ROOT))
            for start, end, value in rust_strings(source):
                prefix = source[max(0, start - 200):start]
                if re.search(r"localization::(?:text|format_text)\(\s*$", prefix):
                    assert value in catalog, f"Missing translation: {path}:{value}"
                    marked += 1
                elif DISPLAY.search(prefix) and re.search(r"[A-Za-z]", value):
                    if value in VERBATIM.get(relative, ()) or (value == "ESC" and relative in KEY_LABEL_FILES):
                        continue
                    remaining.append({"file": str(path.relative_to(ROOT)),
                                      "line": source.count("\n", 0, start) + 1,
                                      "source": value, "offset": start, "end": end})
    return marked, remaining


def self_check():
    source = '''// "comment"
/* outer /* "nested" */ */ '"' "Open" r#"raw \" quoted"# 'a' 'static "last"'''
    assert [value for _, _, value in rust_strings(source)] == ["Open", 'raw " quoted', "last"]
    assert fields("{{literal}} {} {name} {:.2}") == Counter(["0", "name", "1:.2"])
    assert fields("{name} {0} {1:.2}") == fields("{} {name} {:.2}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, help="Write unmarked display literals for review")
    parser.add_argument("--strict", action="store_true", help="Fail on unclassified direct display literals")
    args = parser.parse_args()
    self_check()
    catalog = json.loads(CATALOG.read_text())
    marked, remaining = validate(catalog)
    if args.inventory:
        args.inventory.write_text(json.dumps(remaining, ensure_ascii=False, indent=2) + "\n")
    print(f"Validated {len(catalog)} translation pairs and {marked} marked display sites.")
    print(f"Unclassified direct display literals: {len(remaining)}; review inventory before acceptance.")
    if args.strict and remaining:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
