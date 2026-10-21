extends RefCounted
var text = """body
    Keep literal indentation.
  # This is literal content.
"""
var raw = r"""backslash: \n
  do not normalize these bytes
"""
func paths():
	var name = &"hello"
	var path = ^"Root/Child"
	return [text, raw, name, path]
