
"sizeof" @keyword

[
  "enum"
  "struct"
  "typedef"
  "union"
] @keyword.storage.type

[
  "extern"
  "register"
  (type_qualifier)
  (storage_class_specifier)
] @keyword.storage.modifier

[
  "goto"
  "break"
  "continue"
] @keyword.control

[
  "do"
  "for"
  "while"
] @keyword.control.repeat

[
  "if"
  "else"
  "switch"
  "case"
  "default"
] @keyword.control.conditional

"return" @keyword.control.return

[
  "defined"
  "#define"
  "#elif"
  "#else"
  "#endif"
  "#if"
  "#ifdef"
  "#ifndef"
  "#include"
  (preproc_directive)
] @keyword.directive

(pointer_declarator "*" @type.builtin)
(abstract_pointer_declarator "*" @type.builtin)

[(true) (false)] @constant.builtin.boolean

(enumerator name: (identifier) @type.enum.variant)

(string_literal) @string
(system_lib_string) @string

(null) @constant
(number_literal) @constant.numeric
(char_literal) @constant.character
(escape_sequence) @constant.character.escape


(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function))
(call_expression (argument_list (identifier) @variable))
(function_declarator
  declarator: [(identifier) (field_identifier)] @function)
(parameter_declaration
  declarator: (identifier) @variable.parameter)
(parameter_declaration
  (pointer_declarator
    declarator: (identifier) @variable.parameter))
(preproc_function_def
  name: (identifier) @function.special)

(attribute
  name: (identifier) @attribute)

(field_identifier) @variable.other.member
(statement_identifier) @label
(type_identifier) @type
(primitive_type) @type.builtin
(sized_type_specifier) @type.builtin

(identifier) @variable

(comment) @comment


; Keywords

[
 "try"
 "catch"
 "noexcept"
 "throw"
] @exception @keyword

[
 "class"
 "decltype"
 "explicit"
 "friend"
 "namespace"
 "override"
 "template"
 "typename"
 "using"
 ;"concept"
 ;"requires"
] @keyword

[
  "co_await"
] @keyword.coroutine

[
 "co_yield"
 "co_return"
] @keyword.coroutine.return

[
 "public"
 "private"
 "protected"
 "virtual"
 "final"
] @type.qualifier @keyword

[
 "new"
 "delete"
] @keyword.operator

; Constants

(this) @variable.builtin
;(null "nullptr" @constant.builtin)

(true) @boolean
(false) @boolean

; Literals

(raw_string_literal)  @string

(namespace_identifier) @namespace

;(null "nullptr" @constant.builtin)

(auto) @type.builtin

(operator_name) @function
"operator" @function