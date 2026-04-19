/*
 * Apache Jena adapter. Parses an input file and canonicalises each emitted
 * triple/quad to a line in the fact set.
 *
 * Canonical fact line (matches rdf4j adapter byte-for-byte on agreeing cases):
 *   "<s> <p> <o> ."                — for triple-bearing languages
 *   "<s> <p> <o> <g> ."            — for quad-bearing languages (nq, trig)
 * Blank nodes emit as "_:bN" with N assigned deterministically by first
 * appearance in the parse order. Literals are serialised in their
 * canonical N-Triples form (ESCAPE rules per RFC 8785-style N-Triples).
 */
package zedrdf.factoracles;

import org.apache.jena.graph.Node;
import org.apache.jena.graph.Triple;
import org.apache.jena.riot.Lang;
import org.apache.jena.riot.RDFParser;
import org.apache.jena.riot.system.StreamRDF;
import org.apache.jena.riot.system.StreamRDFBase;
import org.apache.jena.sparql.core.Quad;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

final class JenaParser implements Main.Parser {

    @Override
    public Main.TestOutcome run(String lang, Path suiteDir, Path input) {
        String id = Main.idFromPath(suiteDir, input);
        String sha = Main.sha256Of(input);

        List<String> facts = new ArrayList<>();
        BNodeMap bnodes = new BNodeMap();

        StreamRDF sink = new StreamRDFBase() {
            @Override public void triple(Triple t) {
                facts.add(formatTriple(t, bnodes));
            }
            @Override public void quad(Quad q) {
                facts.add(formatQuad(q, bnodes));
            }
        };

        Lang jenaLang = switch (lang) {
            case "nt"     -> Lang.NTRIPLES;
            case "nq"     -> Lang.NQUADS;
            case "ttl"    -> Lang.TURTLE;
            case "trig"   -> Lang.TRIG;
            case "rdfxml" -> Lang.RDFXML;
            default -> throw new IllegalArgumentException("unsupported lang: " + lang);
        };

        try {
            RDFParser.create()
                     .source(input)
                     .lang(jenaLang)
                     .errorHandler(new org.apache.jena.riot.system.ErrorHandler() {
                         @Override public void warning(String m, long l, long c) { /* silent */ }
                         @Override public void error(String m, long l, long c) {
                             throw new RuntimeException("parse error at " + l + ":" + c + ": " + m);
                         }
                         @Override public void fatal(String m, long l, long c) {
                             throw new RuntimeException("fatal at " + l + ":" + c + ": " + m);
                         }
                     })
                     .parse(sink);
            return new Main.TestOutcome(id, input, sha, true, null, null, facts);
        } catch (Throwable t) {
            return new Main.TestOutcome(id, input, sha, false,
                                        t.getClass().getName(),
                                        String.valueOf(t.getMessage()),
                                        new ArrayList<>());
        }
    }

    // ------------------------------------------------------------------

    private static String formatTriple(Triple t, BNodeMap b) {
        return fmt(t.getSubject(), b) + " "
             + fmt(t.getPredicate(), b) + " "
             + fmt(t.getObject(), b) + " .";
    }

    private static String formatQuad(Quad q, BNodeMap b) {
        String base = fmt(q.getSubject(), b) + " "
                    + fmt(q.getPredicate(), b) + " "
                    + fmt(q.getObject(), b);
        Node g = q.getGraph();
        if (g == null || Quad.isDefaultGraph(g)) {
            return base + " .";
        }
        return base + " " + fmt(g, b) + " .";
    }

    private static String fmt(Node n, BNodeMap b) {
        if (n.isURI()) {
            return "<" + escapeIri(n.getURI()) + ">";
        }
        if (n.isBlank()) {
            return "_:b" + b.idFor(n.getBlankNodeLabel());
        }
        if (n.isLiteral()) {
            String lex = escapeLiteral(n.getLiteralLexicalForm());
            String lang = n.getLiteralLanguage();
            String dt = n.getLiteralDatatypeURI();
            if (lang != null && !lang.isEmpty()) {
                return "\"" + lex + "\"@" + lang.toLowerCase(Locale.ROOT);
            }
            if (dt != null && !dt.equals("http://www.w3.org/2001/XMLSchema#string")) {
                return "\"" + lex + "\"^^<" + escapeIri(dt) + ">";
            }
            return "\"" + lex + "\"";
        }
        return n.toString();
    }

    static String escapeIri(String iri) {
        StringBuilder b = new StringBuilder(iri.length());
        for (int i = 0; i < iri.length(); i++) {
            char c = iri.charAt(i);
            switch (c) {
                case '\\': b.append("\\\\"); break;
                case '<':  b.append("\\u003C"); break;
                case '>':  b.append("\\u003E"); break;
                case '"':  b.append("\\u0022"); break;
                case '{':  b.append("\\u007B"); break;
                case '}':  b.append("\\u007D"); break;
                case '|':  b.append("\\u007C"); break;
                case '^':  b.append("\\u005E"); break;
                case '`':  b.append("\\u0060"); break;
                default:
                    if (c <= 0x20) {
                        b.append(String.format(Locale.ROOT, "\\u%04X", (int) c));
                    } else {
                        b.append(c);
                    }
            }
        }
        return b.toString();
    }

    static String escapeLiteral(String s) {
        StringBuilder b = new StringBuilder(s.length() + 4);
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '\\': b.append("\\\\"); break;
                case '"':  b.append("\\\""); break;
                case '\n': b.append("\\n"); break;
                case '\r': b.append("\\r"); break;
                case '\t': b.append("\\t"); break;
                default:
                    if (c < 0x20) {
                        b.append(String.format(Locale.ROOT, "\\u%04X", (int) c));
                    } else {
                        b.append(c);
                    }
            }
        }
        return b.toString();
    }

    /** Stable, first-appearance-ordered mapping from parser-local bnode labels to b0, b1, ... */
    static final class BNodeMap {
        private final Map<String, Integer> map = new HashMap<>();
        int idFor(String label) {
            return map.computeIfAbsent(label, k -> map.size());
        }
    }
}
