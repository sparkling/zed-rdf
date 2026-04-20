#!/usr/bin/env python3
"""Offline validator for extensions/zed-rdf/extension.toml.

Mirrors the Zed ExtensionManifest schema at
`zed-industries/zed/crates/extension/src/extension_manifest.rs`
(pinned check 2026-04-20). Run before trying `zed: install dev
extension` to catch schema mistakes without opening Zed.
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path


EXT_DIR = Path(__file__).resolve().parent.parent / "extensions" / "zed-rdf"
MANIFEST = EXT_DIR / "extension.toml"
LANG_DIR = EXT_DIR / "languages"

REQUIRED = {"id", "name", "version", "schema_version"}
ALLOWED_TOP_LEVEL = REQUIRED | {
    "description",
    "repository",
    "authors",
    "lib",
    "themes",
    "icon_themes",
    "languages",
    "grammars",
    "language_servers",
    "context_servers",
    "agent_servers",
    "slash_commands",
    "snippets",
    "capabilities",
    "debug_adapters",
    "debug_locators",
    "language_model_providers",
}
ALLOWED_LIB_KIND = {"Rust"}
ALLOWED_LS_KEYS = {"name", "language", "languages", "language_ids", "code_action_kinds"}
ALLOWED_GRAMMAR_KEYS = {"repository", "commit", "rev", "path"}

# Zed ships with a core set of tree-sitter grammars. Extensions may
# reference these by name without declaring them in `[grammars.*]`.
# This list is conservative — extend if a legitimate built-in is
# flagged. See https://zed.dev/docs/extensions/languages for the
# current canonical set.
ZED_BUILTIN_GRAMMARS = {
    "bash",
    "c",
    "cpp",
    "css",
    "go",
    "html",
    "javascript",
    "json",
    "jsonc",
    "markdown",
    "python",
    "regex",
    "ruby",
    "rust",
    "toml",
    "tsx",
    "typescript",
    "xml",
    "yaml",
}


def fatal(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def warn(msg: str) -> None:
    print(f"WARN:  {msg}")


def main() -> None:
    if not MANIFEST.is_file():
        fatal(f"manifest not found: {MANIFEST}")

    try:
        manifest = tomllib.loads(MANIFEST.read_text())
    except tomllib.TOMLDecodeError as e:
        fatal(f"TOML parse failed: {e}")

    missing = REQUIRED - manifest.keys()
    if missing:
        fatal(f"missing required top-level fields: {sorted(missing)}")

    unknown = manifest.keys() - ALLOWED_TOP_LEVEL
    if unknown:
        warn(f"unknown top-level fields (ignored by Zed): {sorted(unknown)}")

    if not isinstance(manifest["id"], str):
        fatal(f"'id' must be a string, got {type(manifest['id']).__name__}")
    if not isinstance(manifest["version"], str):
        fatal(f"'version' must be a string, got {type(manifest['version']).__name__}")
    if manifest["schema_version"] != 1:
        fatal(f"'schema_version' must be 1, got {manifest['schema_version']!r}")

    lib = manifest.get("lib", {})
    if lib:
        if not isinstance(lib, dict):
            fatal(f"'[lib]' must be a table")
        kind = lib.get("kind")
        if kind is not None and kind not in ALLOWED_LIB_KIND:
            fatal(f"'lib.kind' must be one of {sorted(ALLOWED_LIB_KIND)}, got {kind!r}")

    grammars = manifest.get("grammars", {})
    if not isinstance(grammars, dict):
        fatal("'[grammars]' must be a table-of-tables: `[grammars.<name>]`, not `[[grammars]]`")
    for name, entry in grammars.items():
        if not isinstance(entry, dict):
            fatal(f"'grammars.{name}' must be a table")
        if "repository" not in entry:
            fatal(f"'grammars.{name}.repository' is required")
        if "commit" not in entry and "rev" not in entry:
            fatal(f"'grammars.{name}' requires 'commit' (or alias 'rev')")
        unknown_keys = entry.keys() - ALLOWED_GRAMMAR_KEYS
        if unknown_keys:
            warn(f"'grammars.{name}' has unknown keys {sorted(unknown_keys)}")

    language_servers = manifest.get("language_servers", {})
    if not isinstance(language_servers, dict):
        fatal("'[language_servers]' must be a table-of-tables")
    for ls_name, entry in language_servers.items():
        if not isinstance(entry, dict):
            fatal(f"'language_servers.{ls_name}' must be a table")
        unknown_keys = entry.keys() - ALLOWED_LS_KEYS
        if unknown_keys:
            warn(f"'language_servers.{ls_name}' has unknown keys {sorted(unknown_keys)}")
        if "language" in entry and not isinstance(entry["language"], str):
            fatal(
                f"'language_servers.{ls_name}.language' must be a string "
                "(deprecated; prefer plural 'languages = [\"...\"]')"
            )
        if "languages" in entry:
            langs = entry["languages"]
            if not (isinstance(langs, list) and all(isinstance(x, str) for x in langs)):
                fatal(f"'language_servers.{ls_name}.languages' must be a list of strings")
        if "language" not in entry and "languages" not in entry:
            fatal(
                f"'language_servers.{ls_name}' must define 'language' (singular, "
                "deprecated) or 'languages' (plural, list)"
            )

    declared_langs: set[str] = set()
    for ls_entry in language_servers.values():
        for lang in ls_entry.get("languages", []):
            declared_langs.add(lang)
        if single := ls_entry.get("language"):
            declared_langs.add(single)

    config_langs: dict[str, Path] = {}
    if LANG_DIR.is_dir():
        for config_path in LANG_DIR.glob("*/config.toml"):
            try:
                cfg = tomllib.loads(config_path.read_text())
            except tomllib.TOMLDecodeError as e:
                fatal(f"language config parse error in {config_path}: {e}")
            if "name" not in cfg:
                fatal(f"{config_path} missing 'name' field")
            config_langs[cfg["name"]] = config_path

            grammar_ref = cfg.get("grammar")
            if (
                grammar_ref is not None
                and grammar_ref not in grammars
                and grammar_ref not in ZED_BUILTIN_GRAMMARS
            ):
                fatal(
                    f"{config_path} references grammar '{grammar_ref}' but "
                    f"extension.toml does not declare it and it is not a "
                    f"known Zed built-in (available in this extension: "
                    f"{sorted(grammars)})"
                )

    missing_langs = declared_langs - config_langs.keys()
    if missing_langs:
        fatal(
            f"language_servers declares languages {sorted(missing_langs)} but "
            f"no matching languages/<dir>/config.toml with that 'name' exists "
            f"(found: {sorted(config_langs)})"
        )

    extras = config_langs.keys() - declared_langs
    if extras:
        warn(
            f"languages/<dir>/config.toml defines {sorted(extras)} but no "
            f"language_server in extension.toml claims them"
        )

    print(f"OK   extension.toml: id={manifest['id']!r} v{manifest['version']}")
    print(f"OK   grammars: {sorted(grammars)}")
    print(f"OK   language servers: {sorted(language_servers)}")
    print(f"OK   {len(config_langs)} language configs: {sorted(config_langs)}")


if __name__ == "__main__":
    main()
