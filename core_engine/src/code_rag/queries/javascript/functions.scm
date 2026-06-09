; Function declarations
(function_declaration
  name: (identifier) @func_name
  parameters: (formal_parameters) @params) @func_def

; Arrow function assigned to variable
(variable_declarator
  name: (identifier) @var_name
  value: (arrow_function
    parameters: (formal_parameters) @params) @arrow_func)

; Method definition in class
(method_definition
  name: (property_identifier) @method_name
  parameters: (formal_parameters) @params) @method_def

; Export default function
(export_default
  (function_declaration
    name: (identifier) @func_name) @exported_func)
