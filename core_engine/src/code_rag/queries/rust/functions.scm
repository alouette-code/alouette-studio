; Rust function definitions
(function_item
  name: (identifier) @func_name
  parameters: (parameters) @params
  return_type: (_)? @return_type) @func_def

; Public function
(function_item
  visibility: (visibility_modifier) @pub
  name: (identifier) @func_name
  parameters: (parameters) @params) @func_def

; Methods in impl block
(impl_item
  body: (declaration_list
    (function_item
      name: (identifier) @method_name
      parameters: (parameters) @params) @method_def))

; Trait method declarations
(trait_item
  body: (declaration_list
    (function_signature_item
      name: (identifier) @trait_method_name
      parameters: (parameters) @params) @trait_method_def))
