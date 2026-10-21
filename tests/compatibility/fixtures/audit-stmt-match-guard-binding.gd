func run(value = null):
    match value:
        var entry when entry != null: return entry
        _: return null
