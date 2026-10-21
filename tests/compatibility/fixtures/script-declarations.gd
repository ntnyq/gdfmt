class_name Worker
extends Node
@export var count: int = 0
const LIMIT := 10
signal changed(value: int)
enum State { IDLE, ACTIVE = 2, }
class Inner extends RefCounted:
    var data: Dictionary[String, int] = {}
static func compute(value: int, scale = 2, ...rest) -> int:
    return value * scale
