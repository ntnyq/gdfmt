extends RefCounted
func consume(_callback: Callable):
	pass
func make():
	var items = [func(value): return value + 1, func(): return 2]
	var mapping = {"nested": func(value):
		var nested = func(other): return other + value
		return nested.call(2)
	}
	consume(func(value: int = 1) -> int:
		if value > 0:
			return value + 1
		return 0
	)
	return [items, mapping]
var callback = func named(value: int = 2) -> int: return value * 2
