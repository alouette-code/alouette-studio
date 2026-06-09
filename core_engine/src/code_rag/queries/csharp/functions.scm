; Method declarations
(method_declaration
  name: (identifier) @method_name
  parameters: (parameter_list) @params
  return_type: (_) @return_type) @method_def

; Constructor declarations
(constructor_declaration
  name: (identifier) @constructor_name
  parameters: (parameter_list) @params) @constructor_def

; Local function declarations
(local_function_statement
  name: (identifier) @func_name
  parameters: (parameter_list) @params) @func_def
