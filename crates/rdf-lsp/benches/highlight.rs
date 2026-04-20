//! Criterion benchmark: semantic-token highlight on a 10k-line Turtle file.
//!
//! Performance gate (ADR-0025 §5): cold-open highlight ≤ 100 ms.

use criterion::{Criterion, criterion_group, criterion_main};
use rdf_lsp::{Language, semantic_tokens::handle_semantic_tokens};

fn build_10k_turtle() -> String {
    let prefix = "@prefix ex: <http://example.org/> .\n";
    let triple = "ex:subject ex:predicate \"a long object literal that is representative of real data\" .\n";
    let mut s = String::with_capacity(prefix.len() + triple.len() * 10_000);
    s.push_str(prefix);
    for _ in 0..10_000 {
        s.push_str(triple);
    }
    s
}

fn bench_highlight(c: &mut Criterion) {
    let text = build_10k_turtle();
    c.bench_function("highlight_10k_turtle", |b| {
        b.iter(|| {
            let tokens = handle_semantic_tokens(&text, Language::Turtle);
            criterion::black_box(tokens);
        });
    });
}

criterion_group!(benches, bench_highlight);
criterion_main!(benches);
