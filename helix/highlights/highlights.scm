;; highlights.scm for noil

;; Operations (e.g., DELETE, ADD, etc.)
(op_delete) @diff.minus.gutter
(op_add) @diff.plus.gutter
(op_move) @diff.delta.moved
(operation) @constant.character

;; Prefix identifiers (e.g., dwzxz, c444y)
(prefix) @variable.parameter

;; File paths
(file_path) @variable.other.member

;; The colon (separator)
":" @diff.plus.gutter
