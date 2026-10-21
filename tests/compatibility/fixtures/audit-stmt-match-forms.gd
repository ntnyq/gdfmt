func run(value = null):
    match value:
        []: pass
        {}: pass
        [var head, [_, ..], ..] when head != null: pass
        {"key", "other": var entry, ..} when entry != null: pass
        1, 2, 3: pass
        _: pass
