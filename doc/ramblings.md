There's still several large hurdles here.

Is this more efficient than a term splitting algorithm? Can we combine what we
learned about term splitting to make guided mutations?

How can we avoid local minima in the genome space? These diagrams are
conceptually simple. However, it's unclear how to make transformations which
can convert any diagram into every equivalent diagram.

Worse, unlike with binary functions, there are equivalent behavior programs *on
the given inputs* which don't have equivalent behavior without those inputs.

In other words, to fully avoid local minima, we need to have universal neutral
mutations *conditioned on the inputs*. This is even trickier to get right. Probably?

Okay, let's start by building some notation.

Every node in the tree is either a branch or a leaf.

Both branch nodes and leaf nodes contain `terms`.

A term can be:
 - a `reference` which *dynamically* point to earlier locations in the diagram. 
 - a `constant` which has some symbolic value.
 - or, for branch nodes only, `free`, which can be bound to any value.

Branch nodes have two "child" nodes, both of which are optional:
 - a `match` child, which is traversed with each fact that matched the current node.
 - a `refute` child, which is traversed only if no facts matched the current node.


Is there just fundamentally no way to have efficient, contextual neutral mutations?

Actually, this design just isn't very good. It seems elegant, but the action at
a distance makes it kinda a mess.

We're probably better off with a register based system. In that system,
parent-child nodes can be re-ordered as long as their registers don't conflict.

Adjacent parents can be merged if the resgisters would be "shadowed" the same.
Which is somewhat annoying to check, but much better than references.

The size of these trees are almost the same, since we removed the "up"
parameter, and added a destination parameter. The only difference is that the
destination parameter goes on every term, but we only removed the up parameter
from references. However, the space of parameter options can actually be more
effectively controlled, by limiting the number of registers.
