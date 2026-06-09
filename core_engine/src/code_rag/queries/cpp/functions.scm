; Function definitions
(function_definition
  type: (_) @return_type
  declarator: (function_declarator
    declarator: (identifier) @func_name
    parameters: (parameter_list) @params)) @func_def

; Method definitions
(function_definition
  type: (_) @return_type
  declarator: (function_declarator
    declarator: (qualified_identifier
      name: (identifier) @method_name)
    parameters: (parameter_list) @params)) @method_def
