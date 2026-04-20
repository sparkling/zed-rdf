; Turtle / TriG / N3 highlight queries
; Grammar: tree-sitter-turtle (nicowillis)

; Comments
(comment) @comment

; PREFIX and BASE declarations
(prefix_declaration
  prefix: (pname_ns) @namespace
  iri: (iri_reference) @string.special)

(base_declaration
  iri: (iri_reference) @string.special)

; IRIs
(iri_reference) @string.special
(prefixed_name) @namespace

; Blank nodes
(blank_node_label) @constant.builtin
(anon_blank_node) @constant.builtin

; Literals
(rdf_literal
  value: (string_literal_quote) @string
  lang_tag: (lang_tag) @attribute)

(rdf_literal
  value: (string_literal_quote) @string
  datatype: (iri_reference) @type)

(string_literal_quote) @string
(string_literal_single_quote) @string
(string_literal_long_quote) @string
(string_literal_long_single_quote) @string

; Numeric literals
(integer_literal) @number
(decimal_literal) @number
(double_literal) @number

; Boolean literals
(boolean_literal) @constant.builtin

; Keywords
"a" @keyword
"@prefix" @keyword
"@base" @keyword
"PREFIX" @keyword
"BASE" @keyword
"GRAPH" @keyword

; Punctuation
"." @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"[" @punctuation.bracket
"]" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"^^" @operator
