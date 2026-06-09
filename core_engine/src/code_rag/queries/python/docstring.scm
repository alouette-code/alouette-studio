; Lấy docstring ngay phía trên function definition
(expression_statement
  (string) @docstring
  .
  (function_definition) @func_def)

; Lấy comment block phía trên function definition
(comment) @comment
  .
  (function_definition) @func_def
