; Python function definitions
(function_definition
  name: (identifier) @func_name
  parameters: (parameters) @params
  body: (block) @body) @func_def

; Async function definitions
(function_definition
  name: (identifier) @func_name
  parameters: (parameters) @params) @func_def

; Class method definitions
(class_definition
  body: (block
    (function_definition
      name: (identifier) @method_name
      parameters: (parameters) @params) @method_def))

; Decorator (lấy decorator phía trên function)
(decorator
  (identifier) @decorator_name) @decorator
