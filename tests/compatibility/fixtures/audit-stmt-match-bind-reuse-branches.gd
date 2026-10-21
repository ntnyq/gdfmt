func run(value = null):
    match value:
        [var entry]: print(entry)
        {"key": var entry}: print(entry)
