; Function declarations
(function_declaration
  name: (identifier) @func_name
  parameters: (parameter_list) @params
  result: (_)? @return_type) @func_def

; Method declarations (receiver functions)
(method_declaration
  receiver: (parameter_list) @receiver
  name: (field_identifier) @method_name
  parameters: (parameter_list) @params
  result: (_)? @return_type) @method_def
