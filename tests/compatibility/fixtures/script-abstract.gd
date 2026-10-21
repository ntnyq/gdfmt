@abstract
class_name Contract extends RefCounted
@abstract func execute(value: int) -> void
@abstract class Inner:
    @abstract func value() -> int
func concrete(): pass
