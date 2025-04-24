## `MaybeUninit<[u8; N]>` vs `[0u8; N]`

Need to profile differences to see if the `unsafe` is worth it.

Seems like using it as out pointers helps.
