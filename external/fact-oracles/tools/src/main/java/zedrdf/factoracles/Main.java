/*
 * fact-oracles CLI — materialises accept/reject + fact sets for a given W3C
 * suite using either Apache Jena or Eclipse rdf4j, and writes a pinned JSON
 * file conforming to external/fact-oracles/README.md.
 *
 * Reads spec text, not LLM priors (ADR-0019 §1).
 *
 * Output shape is deliberately flat so the Rust side can parse it without
 * pulling a JSON-schema library.
 */
package zedrdf.factoracles;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.SerializationFeature;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.security.MessageDigest;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HexFormat;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Stream;

public final class Main {

    /** Schema version written into every JSON artefact; bumped on breaking shape changes. */
    public static final String SCHEMA_VERSION = "1.0.0";

    public static void main(String[] args) throws Exception {
        Map<String, String> opts = parseArgs(args);
        String lang         = require(opts, "--lang");
        String parserId     = require(opts, "--parser");
        String suiteDir     = require(opts, "--suite-dir");
        String suiteCommit  = require(opts, "--suite-commit");
        String jenaVersion  = opts.getOrDefault("--jena-version", "unknown");
        String rdf4jVersion = opts.getOrDefault("--rdf4j-version", "unknown");
        Path outPath        = Paths.get(require(opts, "--out"));

        Parser parser = switch (parserId) {
            case "jena"  -> new JenaParser();
            case "rdf4j" -> new Rdf4jParser();
            default -> throw new IllegalArgumentException("unknown --parser: " + parserId);
        };

        List<Path> inputs = collectInputs(Paths.get(suiteDir), lang);
        // Deterministic ordering — JSON diffs are stable across runs.
        Collections.sort(inputs);

        List<TestOutcome> outcomes = new ArrayList<>(inputs.size());
        for (Path input : inputs) {
            outcomes.add(parser.run(lang, Paths.get(suiteDir), input));
        }

        String parserVersion = switch (parserId) {
            case "jena"  -> jenaVersion;
            case "rdf4j" -> rdf4jVersion;
            default -> "unknown";
        };

        writeJson(outPath, lang, parserId, parserVersion,
                  suiteCommit, Paths.get(suiteDir), outcomes);
        System.out.printf(Locale.ROOT,
            "wrote %s (%d tests)%n", outPath, outcomes.size());
    }

    // ------------------------------------------------------------------
    // input discovery
    // ------------------------------------------------------------------

    private static List<Path> collectInputs(Path suiteDir, String lang) throws IOException {
        // We prefer to be driven by the upstream manifest, but as of the
        // pinned commit the rdf-tests layout uses per-test files whose
        // extensions are the ground truth. Walking by extension is robust
        // to both vendored and upstream layouts.
        List<String> exts = switch (lang) {
            case "nt"     -> List.of(".nt");
            case "nq"     -> List.of(".nq");
            case "ttl"    -> List.of(".ttl");
            case "trig"   -> List.of(".trig");
            case "rdfxml" -> List.of(".rdf");
            default -> throw new IllegalArgumentException("unknown lang: " + lang);
        };
        List<Path> out = new ArrayList<>();
        try (Stream<Path> walk = Files.walk(suiteDir)) {
            walk.filter(Files::isRegularFile).forEach(p -> {
                String name = p.getFileName().toString().toLowerCase(Locale.ROOT);
                for (String ext : exts) {
                    if (name.endsWith(ext)) {
                        out.add(p);
                        return;
                    }
                }
            });
        }
        return out;
    }

    // ------------------------------------------------------------------
    // JSON emission
    // ------------------------------------------------------------------

    private static void writeJson(Path outPath,
                                  String lang,
                                  String parserId,
                                  String parserVersion,
                                  String suiteCommit,
                                  Path suiteDir,
                                  List<TestOutcome> outcomes) throws IOException {
        ObjectMapper mapper = new ObjectMapper();
        mapper.configure(SerializationFeature.ORDER_MAP_ENTRIES_BY_KEYS, true);
        ObjectNode root = mapper.createObjectNode();
        root.put("schema_version", SCHEMA_VERSION);
        root.put("lang", lang);
        root.put("parser", parserId);
        root.put("parser_version", parserVersion);
        root.put("suite_commit", suiteCommit);
        root.put("generated_at_utc", Instant.now().toString());

        ArrayNode cases = root.putArray("cases");
        for (TestOutcome o : outcomes) {
            ObjectNode c = cases.addObject();
            c.put("id", o.id);
            c.put("input_path", suiteDir.relativize(o.inputPath).toString().replace('\\', '/'));
            c.put("input_sha256", o.inputSha256);
            c.put("accepted", o.accepted);
            if (!o.accepted) {
                c.put("error_class", o.errorClass == null ? "" : o.errorClass);
                c.put("error_message", o.errorMessage == null ? "" : o.errorMessage);
            }
            ArrayNode facts = c.putArray("facts");
            // Deterministic fact ordering.
            Collections.sort(o.facts);
            for (String f : o.facts) {
                facts.add(f);
            }
            c.put("fact_count", o.facts.size());
        }

        Files.createDirectories(outPath.getParent());
        mapper.writerWithDefaultPrettyPrinter().writeValue(outPath.toFile(), root);
    }

    // ------------------------------------------------------------------
    // support types
    // ------------------------------------------------------------------

    /** Flat result for a single suite entry. */
    static final class TestOutcome {
        final String id;
        final Path inputPath;
        final String inputSha256;
        final boolean accepted;
        final String errorClass;
        final String errorMessage;
        final List<String> facts;

        TestOutcome(String id, Path inputPath, String inputSha256,
                    boolean accepted, String errorClass, String errorMessage,
                    List<String> facts) {
            this.id = id;
            this.inputPath = inputPath;
            this.inputSha256 = inputSha256;
            this.accepted = accepted;
            this.errorClass = errorClass;
            this.errorMessage = errorMessage;
            this.facts = facts;
        }
    }

    /** Parser adapter. Implementations must be side-effect free beyond file IO. */
    interface Parser {
        TestOutcome run(String lang, Path suiteDir, Path input);
    }

    // ------------------------------------------------------------------
    // CLI helpers
    // ------------------------------------------------------------------

    private static Map<String, String> parseArgs(String[] args) {
        Map<String, String> out = new TreeMap<>();
        for (int i = 0; i < args.length; i++) {
            String a = args[i];
            if (!a.startsWith("--")) {
                throw new IllegalArgumentException("unexpected positional arg: " + a);
            }
            if (i + 1 >= args.length) {
                throw new IllegalArgumentException("missing value for " + a);
            }
            out.put(a, args[++i]);
        }
        return out;
    }

    private static String require(Map<String, String> opts, String key) {
        String v = opts.get(key);
        if (v == null) {
            throw new IllegalArgumentException("missing required arg: " + key);
        }
        return v;
    }

    static String sha256Of(Path p) {
        try (InputStream in = Files.newInputStream(p)) {
            MessageDigest md = MessageDigest.getInstance("SHA-256");
            byte[] buf = new byte[8192];
            int n;
            while ((n = in.read(buf)) > 0) {
                md.update(buf, 0, n);
            }
            return HexFormat.of().formatHex(md.digest());
        } catch (Exception e) {
            return "";
        }
    }

    static String idFromPath(Path suiteDir, Path input) {
        // Stable identifier: path relative to the suite root, no extension.
        String rel = suiteDir.relativize(input).toString().replace('\\', '/');
        int dot = rel.lastIndexOf('.');
        return dot < 0 ? rel : rel.substring(0, dot);
    }
}
