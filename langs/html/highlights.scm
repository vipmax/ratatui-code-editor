(tag_name) @type
(erroneous_end_tag_name) @tag.error
(doctype) @constant
(attribute_name) @variable
(attribute_value) @string
(comment) @comment

[
  "<"
  ">"
  "</"
  "/>"
] @punctuation.bracket

(script_element
  (raw_text) @injection.content.javascript)
 (#set! injection.language "javascript")

(style_element
  (raw_text) @injection.content.css)
 (#set! injection.language "css")