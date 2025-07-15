[
  (atx_heading)
  (setext_heading)
] @type

[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
  (list_marker_dot)
  (list_marker_parenthesis)
] @punctuation.list_marker

(fenced_code_block
  (info_string
    (language) @string))

((inline) @injection.content.markdown-inline
 (#set! injection.language "markdown-inline"))

((html_block) @injection.content
  (#set! injection.language "html"))

((minus_metadata) @injection.content (#set! injection.language "yaml"))

((plus_metadata) @injection.content (#set! injection.language "toml"))

;; Rust
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "rust"))
  (code_fence_content) @injection.content.rust)

;; JavaScript
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "javascript"))
  (code_fence_content) @injection.content.javascript)

;; TypeScript
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "typescript"))
  (code_fence_content) @injection.content.typescript)

;; Python
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "python"))
  (code_fence_content) @injection.content.python)

;; TOML
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "toml"))
  (code_fence_content) @injection.content.toml)

;; JSON
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "json"))
  (code_fence_content) @injection.content.json)

;; YAML
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "yaml"))
  (code_fence_content) @injection.content.yaml)

;; Bash
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "shell"))
  (code_fence_content) @injection.content.shell)

;; C
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "c"))
  (code_fence_content) @injection.content.c)

;; C++
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "cpp"))
  (code_fence_content) @injection.content.cpp)

;; C#
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "c_sharp"))
  (code_fence_content) @injection.content.c_sharp)

;; Go
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "go"))
  (code_fence_content) @injection.content.go)

;; Java
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "java"))
  (code_fence_content) @injection.content.java)

;; HTML
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "html"))
  (code_fence_content) @injection.content.html)

;; CSS
(fenced_code_block
  (info_string
    (language) @injection.language
    (#match? @injection.language "css"))
  (code_fence_content) @injection.content.css)
