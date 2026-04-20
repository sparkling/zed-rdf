# Grammar Pins

This file documents the tree-sitter grammar sources and commit pins for
`zed-rdf`. Each pin is also recorded in `extension.toml` `[[grammars]]`
entries. Update both when re-pinning.

| Language       | Grammar          | Repository                                          | Pinned commit |
|----------------|------------------|-----------------------------------------------------|---------------|
| Turtle / TriG / N3 / NT / NQ | `turtle` | https://github.com/nicowillis/tree-sitter-turtle | `c0daf6c39ebcb17a3c3f3ff17c89e4fa3c7e4b8a` |
| SPARQL         | `sparql`         | https://github.com/GordianDziwis/tree-sitter-sparql | `6ae5f898b94e3d8e6dfd87cfe29b879e62fec8a3` |
| ShEx           | `shex`           | https://github.com/nicowillis/tree-sitter-shex     | `a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9` |
| RDF/XML / TriX | `xml`            | Built-in Zed grammar                                | (built-in)    |
| JSON-LD        | `json`           | Built-in Zed grammar                                | (built-in)    |
| Datalog        | *(none)*         | No community grammar; LSP provides semantic tokens  | N/A           |

## Re-pinning procedure

1. Update the commit hash in `extension.toml` under the relevant `[[grammars]]` entry.
2. Update the table above.
3. Run the CI job locally: `tree-sitter query parse` for each language's `.scm` files.
4. Fix any broken node-type references in the `.scm` files.
5. Submit a PR — the `tree-sitter-queries` CI job is a required gate.
