; SPARQL highlight queries.
; Grammar: GordianDziwis/tree-sitter-sparql (commit 1ef52d3).
;
; Structural nodes are used in preference to string-literal keywords,
; because the grammar accepts keywords case-insensitively and Zed only
; highlights exact literal matches.

; Comments
(comment) @comment

; Prologue
(base_declaration) @keyword
(prefix_declaration) @keyword

; Query forms
(select_query) @function
(construct_query) @function
(describe_query) @function
(ask_query) @function
(sub_select) @function

; Update operations
(insert_data) @function
(delete_data) @function
(delete_where) @function
(modify) @function
(load) @function
(clear) @function
(drop) @function
(create) @function
(add) @function
(move) @function
(copy) @function

; Graph patterns
(optional_graph_pattern) @keyword
(minus_graph_pattern) @keyword
(filter) @keyword
(bind) @keyword
(values_clause) @keyword
(service_graph_pattern) @keyword
(graph_graph_pattern) @keyword
(exists_func) @keyword
(not_exists_func) @keyword

; Solution modifiers
(group_clause) @keyword
(having_clause) @keyword
(order_clause) @keyword
(limit_clause) @keyword
(offset_clause) @keyword

; Functions
(function_call) @function.call
(build_in_function) @function.builtin
(aggregate) @function.builtin
(regex_expression) @function.builtin
(substring_expression) @function.builtin
(string_replace_expression) @function.builtin

; Variables
(var) @variable

; Namespaces, IRIs, prefixed names
(namespace) @namespace
(iri_reference) @string.special
(prefixed_name) @type

; Blank nodes
(blank_node_label) @variable
(anon) @variable

; Literals
(string) @string
(lang_tag) @string.special
(boolean_literal) @boolean
(integer) @number
(decimal) @number
(double) @number
(nil) @constant

; Punctuation
"." @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
