func test(value):
	match value:
		[var first, ..] when first > 0:
			return first
