#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_diff::Parser;
use rdf_xml::RdfXmlParser;

fuzz_target!(|data: &[u8]| {
    match RdfXmlParser.parse(data) {
        Ok(outcome) => {
            assert!(!outcome.warnings.fatal, "Ok() path must never emit a fatal warning");
        }
        Err(diag) => {
            assert!(diag.fatal, "Err() path must carry fatal: true");
            assert!(!diag.messages.is_empty(), "Err() path must carry at least one message");
        }
    }
});
