#[test]
fn probe() {
    for c in ['\u{202E}', '\u{FFFD}', '\u{200B}', '\u{FE0F}', '\u{180E}', '\u{2028}'] {
        let h = format!("a{}b.example", c);
        match idna::domain_to_ascii_strict(&h) {
            Ok(s) => eprintln!("U+{:04X} OK: {}", c as u32, s),
            Err(e) => eprintln!("U+{:04X} ERR: {:?}", c as u32, e),
        }
    }
}
