; ShEx compact syntax highlight queries
; Grammar: tree-sitter-shex (nicowillis)

; Comments
(comment) @comment

; Keywords
["PREFIX" "BASE" "START" "CLOSED" "EXTENDS" "AND" "OR" "NOT"] @keyword
["IRI" "LITERAL" "NONLITERAL" "BNODE" "ABSTRACT" "EXTRA"] @keyword

; IRIs and prefixed names
(iri_reference) @string.special
(prefixed_name) @namespace
(prefix_declaration prefix: (pname_ns) @namespace)

; Literals
(string_literal_quote) @string
(string_literal_long_quote) @string

; Shape labels
(shape_label) @type

; Cardinality
["+" "*" "?"] @operator
("{" "}") @punctuation.bracket

; Punctuation
"." @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"@" @keyword
"^" @operator
"&" @operator
"|" @operator
