func run(values):
    var total = 0; const STEP = 1
    for value: int in values:
        if value < 0: continue
        elif value == 0: break
        else: total += value
    while total < 20:
        total += STEP
    assert(total >= 0, 'negative')
    breakpoint
    return total
