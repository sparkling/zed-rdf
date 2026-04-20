//! Zed extension entry point for zed-rdf.
//!
//! This extension's only job is to locate and launch `rdf-lsp` as the
//! Language Server. All LSP intelligence lives in the `rdf-lsp` binary;
//! this thin launcher is intentionally kept minimal so that future
//! `zed_extension_api` migrations require minimal changes.

use zed_extension_api::{self as zed, LanguageServerId, Result};

struct ZedRdfExtension;

impl zed::Extension for ZedRdfExtension {
    fn new() -> Self {
        ZedRdfExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = worktree
            .which("rdf-lsp")
            .ok_or("rdf-lsp binary not found on PATH")?;

        Ok(zed::Command {
            command: server_path,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(ZedRdfExtension);
