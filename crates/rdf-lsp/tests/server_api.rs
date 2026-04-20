//! Integration tests that verify the current rdf-lsp stub API compiles and exports
//! the required symbols. These tests pass without any implementation beyond the stub.

/// Verify that `run_server` is exported as a callable function symbol.
///
/// This test does not *call* `run_server` (which would block on stdin), it
/// merely coerces it to a function pointer, which is a compile-time + link-time
/// check that the symbol exists and has the expected signature `fn()`.
#[test]
fn run_server_symbol_exists() {
    let _f: fn() = rdf_lsp::run_server;
}

/// Trivially passes: the act of successfully compiling and linking this test
/// binary proves that the `rdf_lsp` crate builds without errors.
#[test]
fn crate_builds() {}
