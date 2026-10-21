func inspect(value):
    match value:
        [var head, ..] when head > 0: return head
        {'key': var found, ..}: return found
        1, 2: pass
        _: return null
