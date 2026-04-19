#!/usr/bin/env python3
"""
Zero-dependency validator for fact-oracle JSON files.

Mirrors the schema documented in external/fact-oracles/README.md so the
workflow fails fast if the Java tool ever emits a shape the Rust side
cannot consume.

Usage:
    python3 validate_schema.py <path-to-json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

SCHEMA_VERSION_MAJOR = 1

REQUIRED_TOP = {
    "schema_version": str,
    "lang": str,
    "parser": str,
    "parser_version": str,
    "suite_commit": str,
    "generated_at_utc": str,
    "cases": list,
}

LANGS = {"nt", "nq", "ttl", "trig", "rdfxml"}
PARSERS = {"jena", "rdf4j"}


def fail(msg: str) -> None:
    print(f"::error title=fact-oracle schema violation::{msg}", file=sys.stderr)
    sys.exit(1)


def validate(path: Path) -> None:
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        fail(f"{path}: not valid JSON: {e}")

    if not isinstance(doc, dict):
        fail(f"{path}: top-level must be an object")

    for key, typ in REQUIRED_TOP.items():
        if key not in doc:
            fail(f"{path}: missing required top-level key '{key}'")
        if not isinstance(doc[key], typ):
            fail(f"{path}: key '{key}' must be {typ.__name__}")

    major = doc["schema_version"].split(".", 1)[0]
    if major != str(SCHEMA_VERSION_MAJOR):
        fail(f"{path}: schema_version major must be {SCHEMA_VERSION_MAJOR}, got {doc['schema_version']}")

    if doc["lang"] not in LANGS:
        fail(f"{path}: lang must be one of {sorted(LANGS)}, got {doc['lang']!r}")
    if doc["parser"] not in PARSERS:
        fail(f"{path}: parser must be one of {sorted(PARSERS)}, got {doc['parser']!r}")

    for i, case in enumerate(doc["cases"]):
        if not isinstance(case, dict):
            fail(f"{path}: cases[{i}] must be an object")
        for key, typ in (
            ("id", str),
            ("input_path", str),
            ("input_sha256", str),
            ("accepted", bool),
            ("facts", list),
            ("fact_count", int),
        ):
            if key not in case:
                fail(f"{path}: cases[{i}] missing '{key}'")
            if not isinstance(case[key], typ):
                fail(f"{path}: cases[{i}].{key} must be {typ.__name__}")
        if not case["accepted"]:
            for key in ("error_class", "error_message"):
                if key not in case or not isinstance(case[key], str):
                    fail(f"{path}: cases[{i}] rejected entries must carry string '{key}'")
        if case["fact_count"] != len(case["facts"]):
            fail(f"{path}: cases[{i}].fact_count ({case['fact_count']}) != len(facts) ({len(case['facts'])})")
        for j, f in enumerate(case["facts"]):
            if not isinstance(f, str):
                fail(f"{path}: cases[{i}].facts[{j}] must be a string")

    print(f"ok: {path} ({len(doc['cases'])} cases)")


def main() -> None:
    if len(sys.argv) != 2:
        print("usage: validate_schema.py <path-to-json>", file=sys.stderr)
        sys.exit(2)
    validate(Path(sys.argv[1]))


if __name__ == "__main__":
    main()
