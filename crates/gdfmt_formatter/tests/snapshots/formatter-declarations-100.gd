@tool
extends RefCounted
signal changed(value: int, label: String)
enum Status {READY = 1, BUSY = 2, DONE,}
const DEFAULT: int = 1
static var count: int = 0
@export
var value: int = DEFAULT:
	get():
		return value
	set(next):
		value = next;
		changed.emit(value, "value")
var delegated: int:
	get = read_delegated,
	set = write_delegated
func read_delegated() -> int:
	return value
func write_delegated(next):
	value = next
class Inner extends RefCounted:
	var data: Array[int] = [1, 2]
	static func create(first: int = 1, second: int = 2) -> Array[int]:
		return [first, second]
