
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
(property_name) @function

(string_value) @string
((color_value) "#") @string.special
(color_value) @string.special

(integer_value) @constant.numeric.integer
(float_value) @constant.numeric.float
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