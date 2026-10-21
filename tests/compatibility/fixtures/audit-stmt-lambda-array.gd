func run(value = null):
    var outer = 1
    var callbacks = [func():
        var outer = 2
        return outer
    ]
