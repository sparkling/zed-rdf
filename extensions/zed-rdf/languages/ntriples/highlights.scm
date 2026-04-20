; Turtle / TriG / N-Triples / N-Quads / N3 highlight queries.
; Grammar: GordianDziwis/tree-sitter-turtle (commit 7f789ea).

; Comments
(comment) @comment

; Directives (@prefix, @base, SPARQL-style PREFIX/BASE)
(prefix_id) @keyword
(base) @keyword
(sparql_prefix) @keyword
(sparql_base) @keyword

; Namespace prefix declarations (ex: <http://…>) — tag the prefix part
(namespace) @namespace

; IRIs and prefixed names
(iri_reference) @string.special
(prefixed_name) @type

; Blank node labels (_:b0, [])
(blank_node_label) @variable
(anon) @variable

; Literals
(string) @string
(lang_tag) @string.special
(boolean_literal) @boolean
(integer) @number
(decimal) @number
(double) @number

; Punctuation
"." @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"[" @punctuation.bracket
"]" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
"^^" @operator
