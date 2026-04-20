; SPARQL highlight queries
; Grammar: tree-sitter-sparql (GordianDziwis)

; Comments
(comment) @comment

; Keywords — query forms
["SELECT" "CONSTRUCT" "ASK" "DESCRIBE"] @keyword
["WHERE" "FROM" "NAMED"] @keyword
["OPTIONAL" "UNION" "MINUS" "FILTER" "BIND" "VALUES" "SERVICE"] @keyword
["GROUP" "BY" "HAVING" "ORDER" "LIMIT" "OFFSET"] @keyword
["DISTINCT" "REDUCED" "AS" "IN" "NOT" "EXISTS"] @keyword
["INSERT" "DELETE" "LOAD" "CLEAR" "DROP" "CREATE" "COPY" "MOVE" "ADD"] @keyword
["WITH" "USING" "DEFAULT" "ALL" "SILENT"] @keyword
["GRAPH" "BASE" "PREFIX"] @keyword

; Aggregate and built-in functions
["COUNT" "SUM" "MIN" "MAX" "AVG" "SAMPLE" "GROUP_CONCAT"] @function.builtin
["STR" "LANG" "DATATYPE" "IRI" "URI" "BNODE" "RAND" "ABS" "CEIL"] @function.builtin
["FLOOR" "ROUND" "CONCAT" "STRLEN" "UCASE" "LCASE" "ENCODE_FOR_URI"] @function.builtin
["CONTAINS" "STRSTARTS" "STRENDS" "STRBEFORE" "STRAFTER"] @function.builtin
["YEAR" "MONTH" "DAY" "HOURS" "MINUTES" "SECONDS" "TIMEZONE" "TZ" "NOW"] @function.builtin
["UUID" "STRUUID" "MD5" "SHA1" "SHA256" "SHA384" "SHA512"] @function.builtin
["COALESCE" "IF" "STRLANG" "STRDT" "SAMETERM"] @function.builtin
["ISIRI" "ISURI" "ISBLANK" "ISLITERAL" "ISNUMERIC"] @function.builtin
["REGEX" "SUBSTR" "REPLACE" "SEPARATOR"] @function.builtin

; Variables
(var) @variable

; IRIs
(iri_reference) @string.special
(prefixed_name) @namespace
(prefix_declaration prefix: (pname_ns) @namespace)

; Literals
(rdf_literal value: (string_literal_quote) @string)
(string_literal_quote) @string
(string_literal_long_quote) @string

; Numeric literals
(integer_literal) @number
(decimal_literal) @number
(double_literal) @number

; Boolean
(boolean_literal) @constant.builtin

; Blank nodes
(blank_node_label) @constant.builtin
(anon_blank_node) @constant.builtin

; Operators
["=" "!=" "<" ">" "<=" ">=" "+" "-" "*" "/" "!" "&&" "||"] @operator
["|" "/" "^" "?" "+" "*"] @operator

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
"^^" @operator
