extends RefCounted
func run(value : int):
    var total:=0
    if value>2: total+=1; total+=2;;
    elif value==2: total=2
    else: total=0
    for item : int in [ 1,2,3 ]:
        if item==value: continue
        total+=item
    while total>10:
        total-=1
        if total==12: break
    match [ value,total ]:
        [0, var result] when result>=0: total=result
        [1, ..], [2, ..]: pass
        _: breakpoint
    assert( total>=0, "negative", )
    return total
