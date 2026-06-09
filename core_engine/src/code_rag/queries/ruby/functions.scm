; Method definitions
(method
  name: (identifier) @method_name
  parameters: (method_parameters) @params
  body: (body_statement) @body) @method_def

; Singleton method (def self.method)
(singleton_method
  object: (_) @object
  name: (identifier) @method_name
  parameters: (method_parameters) @params) @singleton_method

; Block with parameters
(block
  parameters: (block_parameters) @block_params
  body: (body_statement) @body) @block
