
(comment) @comment

[
 (tag_name)
 (nesting_selector)
 (universal_selector)
] @identifier

(attribute_name) @attribute
(class_name) @identifier
(feature_name) @variable.other.member
(function_name) @function
(id_name) @identifier
(namespace_name) @namespace
(property_name) @variable

(string_value) @string
((color_value) "#") @string
(color_value) @string

(integer_value) @constant
(float_value) @constant
(plain_value) @constant

[
 "@charset"
 "@import"
 "@keyframes"
 "@media"
 "@namespace"
 "@supports"
 (at_keyword)
 (from)
 (important)
 (to)
] @keyword