func run(value = null):
    var cb = func():
        var captured = 1
        return func():
            var captured = 2
            return captured
