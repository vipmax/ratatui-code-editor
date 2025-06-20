(
  (class_definition
    body: (block
    (function_definition
      name: (identifier) @method @test-name)))
  (#match? @method "test")
)

(
  class_definition name: (identifier) @test-name
  (#match? @test-name "[Tt]est")
)