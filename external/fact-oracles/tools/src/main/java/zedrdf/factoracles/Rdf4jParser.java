/*
 * Eclipse rdf4j adapter. Produces fact lines that match JenaParser
 * byte-for-byte when the two parsers agree; divergences on any byte
 * become a diff harness signal on the Rust side.
 */
package zedrdf.factoracles;

import org.eclipse.rdf4j.model.BNode;
import org.eclipse.rdf4j.model.IRI;
import org.eclipse.rdf4j.model.Literal;
import org.eclipse.rdf4j.model.Resource;
import org.eclipse.rdf4j.model.Statement;
import org.eclipse.rdf4j.model.Value;
import org.eclipse.rdf4j.rio.RDFFormat;
import org.eclipse.rdf4j.rio.RDFHandler;
import org.eclipse.rdf4j.rio.RDFHandlerException;
import org.eclipse.rdf4j.rio.RDFParser;
import org.eclipse.rdf4j.rio.Rio;

import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

final class Rdf4jParser implements Main.Parser {

    @Override
    public Main.TestOutcome run(String lang, Path suiteDir, Path input) {
        String id = Main.idFromPath(suiteDir, input);
        String sha = Main.sha256Of(input);

        List<String> facts = new ArrayList<>();
        JenaParser.BNodeMap bnodes = new JenaParser.BNodeMap();
        boolean isQuads = "nq".equals(lang) || "trig".equals(lang);

        RDFFormat format = switch (lang) {
            case "nt"     -> RDFFormat.NTRIPLES;
            case "nq"     -> RDFFormat.NQUADS;
            case "ttl"    -> RDFFormat.TURTLE;
            case "trig"   -> RDFFormat.TRIG;
            case "rdfxml" -> RDFFormat.RDFXML;
            default -> throw new IllegalArgumentException("unsupported lang: " + lang);
        };

        RDFParser parser = Rio.createParser(format);
        // Strict: treat warnings from the parser as accept/reject signals
        // the spec actually cares about.
        parser.getParserConfig().set(
            org.eclipse.rdf4j.rio.helpers.BasicParserSettings.FAIL_ON_UNKNOWN_DATATYPES, false);

        parser.setRDFHandler(new RDFHandler() {
            @Override public void startRDF() { }
            @Override public void endRDF() { }
            @Override public void handleNamespace(String prefix, String uri) { }
            @Override public void handleComment(String comment) { }
            @Override public void handleStatement(Statement st) throws RDFHandlerException {
                facts.add(formatStatement(st, bnodes, isQuads));
            }
        });

        try (InputStream in = Files.newInputStream(input)) {
            // Base IRI per the W3C manifests convention: use the file URL.
            String baseIri = input.toUri().toString();
            parser.parse(in, baseIri);
            return new Main.TestOutcome(id, input, sha, true, null, null, facts);
        } catch (Throwable t) {
            return new Main.TestOutcome(id, input, sha, false,
                                        t.getClass().getName(),
                                        String.valueOf(t.getMessage()),
                                        new ArrayList<>());
        }
    }

    // ------------------------------------------------------------------

    private static String formatStatement(Statement st, JenaParser.BNodeMap b, boolean isQuads) {
        String base = fmt(st.getSubject(), b) + " "
                    + fmt(st.getPredicate(), b) + " "
                    + fmt(st.getObject(), b);
        if (isQuads && st.getContext() != null) {
            return base + " " + fmt(st.getContext(), b) + " .";
        }
        return base + " .";
    }

    private static String fmt(Value v, JenaParser.BNodeMap b) {
        if (v instanceof IRI iri) {
            return "<" + JenaParser.escapeIri(iri.stringValue()) + ">";
        }
        if (v instanceof BNode bn) {
            return "_:b" + b.idFor(bn.getID());
        }
        if (v instanceof Literal lit) {
            String lex = JenaParser.escapeLiteral(lit.getLabel());
            if (lit.getLanguage().isPresent()) {
                return "\"" + lex + "\"@" + lit.getLanguage().get().toLowerCase(Locale.ROOT);
            }
            IRI dt = lit.getDatatype();
            if (dt != null && !dt.stringValue().equals("http://www.w3.org/2001/XMLSchema#string")) {
                return "\"" + lex + "\"^^<" + JenaParser.escapeIri(dt.stringValue()) + ">";
            }
            return "\"" + lex + "\"";
        }
        if (v instanceof Resource r) {
            return r.stringValue();
        }
        return v.stringValue();
    }
}
