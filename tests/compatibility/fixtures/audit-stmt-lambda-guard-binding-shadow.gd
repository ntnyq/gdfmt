func run(value = null):
    match value:
        var entry when (func():
            var entry = 1
            return entry
        ).call(): pass
