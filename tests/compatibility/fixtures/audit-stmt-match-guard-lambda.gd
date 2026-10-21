func run(value = null):
    match value:
        _ when (func(): return true).call(): pass
