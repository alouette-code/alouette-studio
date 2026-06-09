; Function declarations
(function_declaration
  name: (identifier) @func_name
  parameters: (formal_parameters) @params
  return_type: (_)? @return_type) @func_def

; Arrow function with type
(variable_declarator
  name: (identifier) @var_name
  value: (arrow_function
    parameters: (formal_parameters) @params) @arrow_func)

; Method in interface
(interface_declaration
  body: (object_type
    (method_signature
      name: (property_identifier) @method_name
      parameters: (formal_parameters) @params) @method_sig))

; Method in class
(method_definition
  name: (property_identifier) @method_name
  parameters: (formal_parameters) @params) @method_def
