; Method declarations
(method_declaration
  name: (identifier) @method_name
  parameters: (formal_parameters) @params
  return_type: (_) @return_type) @method_def

; Constructor declarations
(constructor_declaration
  name: (identifier) @constructor_name
  parameters: (formal_parameters) @params) @constructor_def

; Interface method declarations
(interface_declaration
  body: (interface_body
    (method_declaration
      name: (identifier) @method_name
      parameters: (formal_parameters) @params) @interface_method))
