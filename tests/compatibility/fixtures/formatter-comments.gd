## A documented script.
extends RefCounted
#region data
## Values keep their own comments.
var values  = [ 1, # first
  # leading second
  2,
  # dangling in the list
]
var empty = { # opening
  # dangling in an empty dictionary
}
#endregion

func read_value( # opening parameter list
  index : int = 0 # parameter
) -> int: # function header
    # leading return
    return values [ index ] # return suffix
    # dangling function comment

# leading next declaration
var callback = func( value ): # lambda header
    return value # lambda return
# eof comment
