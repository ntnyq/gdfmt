extends Node

func consume(callback: Callable, _value = 0) -> void:
	callback.call()

func noop() -> void:
	pass

func return_call() -> void:
	return noop()

func return_grouped_call() -> void:
	return (str(1))

func run() -> void:
	consume(
		func():
			var value = 1
			print(value))
	consume(
		func():
			print("next"), 1)
	var sentinel = 2
	print(sentinel)
