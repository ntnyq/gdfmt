var values: Dictionary[String, int] = {"one": 1}
func lookup(data: Dictionary[String, int]) -> Dictionary[String, int]:
    var cast = data as Dictionary[String, int]
    if cast is Dictionary[String, int]: return cast
    return {}
