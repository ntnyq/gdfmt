var value:
    get: return 1
    set(next): print(next)
var delegated: get = read_value, set = write_value
