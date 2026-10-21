extends RefCounted

func conditions(first: bool, second: bool) -> bool:
	if first \
		# Continue the condition after an own-line comment.
		and second:
		return true
	return false

func arithmetic(value: int) -> int:
	var result = value \
		# Keep the addition in the same logical statement.
		+ 1 \
		# Keep the multiplication in the same expression.
		* 2
	return result \
		# A continued return expression.
		+ 3

func nested(value: int) -> Array:
	return [value \
		# Redundant explicit continuations inside delimiters are valid too.
		+ 1, value]
