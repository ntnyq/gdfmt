extends RefCounted
signal empty
signal changed(value: int,)
enum { FIRST, SECOND = 1 << 2, }
enum State { A = -1, B, }
const LIMIT: int = 3
var inferred := 2
static var shared: int = 0
class Inner extends RefCounted:
    const X := 1
    signal ready_now
    enum Kind { A, B }
