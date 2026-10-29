#!/usr/bin/env python3
"""Fact `commands`: Tauri commands registered in Rust vs calls made by ui/src/lib/api.ts."""
import json, re
from pathlib import Path

rust = set(re.findall(r"commands::(\w+)", re.search(r"generate_handler!\[(.*?)\]", Path("src-tauri/src/lib.rs").read_text(), re.S).group(1)))
ui = set(re.findall(r"invoke(?:<[^>]*>)?\(\s*'(\w+)'", Path("ui/src/lib/api.ts").read_text()))
yn = lambda b: "oui" if b else "NON"
print(json.dumps([{"name": n, "columns": {"Rust": yn(n in rust), "api.ts": yn(n in ui)}} for n in sorted(rust | ui)]))
