⚠️⚠️⚠️ **THIS IS A BIG UGLY HACK** ⚠️⚠️⚠️

This is stil a WIP only made public to share with some people.
I'm pretty happy with the use exposed api of the proc-macro so far but
1. it needs some more features and test
2. the proc macro is _very_ ugly and likely fragile

-------

## Bitstuff
1️⃣0️⃣1️⃣1️⃣0️⃣1️⃣0️⃣👉📦

A little crate for defining bit-packed structs and enums.
Its goals are to _look_ as much like normal rust structs as possible and generate code which is easy to read, can have nice docs, and checks for / disallows invalid bitpatterns.
Mainly intended for specifying registers for use in embedded or OSDev, but should be useable for any bitfield.

See [pl011-uart-registers](https://github.com/HayleyDeckers/pl011-uart-registers) for a real-world use-case
