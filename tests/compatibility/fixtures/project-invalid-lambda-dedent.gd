extends Node

func consume(callback: Callable) -> void:
	callback.call()

func run() -> void:
	consume(
			func():
				pass
			)
