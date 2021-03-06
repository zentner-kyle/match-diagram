There's still several large hurdles here.

Is this more efficient than a term splitting algorithm? Can we combine what we
learned about term splitting to make guided mutations?

How can we avoid local maxima in the genome space? These diagrams are
conceptually simple. However, it's unclear how to make transformations which
can convert any diagram into every equivalent diagram.

Worse, unlike with binary functions, there are equivalent behavior programs *on
the given inputs* which don't have equivalent behavior without those inputs.

In other words, to fully avoid local maxima, we need to have universal neutral
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

Registers also make allowing loops easy, since the state can be clearly snapshot.

If we evaluate trees in a breadth first manner, accumulating sets of snapshots
to each node, can we easily determine neutral mutations? Not in general. We can
easily determine if the next layer of register snapshots change, but there may
be neutral mutations where some of the intermediate snapshots change.

The critical property of EBDDIN we want to replicate is that a single example
can be solved, and that is guaranteed to be able to re-integrate into the rest
of the diagram.

Splitting off and solving a problem is pretty easy in this design, since
constant match nodes are sufficient to differentiate every input example.

Maybe we should just start designing neutral mutations.

Non-size changing mutations:
 - Replace a constant with a register match, as long as the incoming register
   snapshots always contain that constant in the register.
 - Inverse of above.
 - Move a register match to a new register, as long as the incoming register
   snapshots always contain the same values as the original register.
 - Change a register output, as long as each outgoing set of register snapshots
   don't change.
 - Change a constant with another constant, as long as each outgoing set of
   register snapshots don't change.

Size changing mutations:
 - Replace an edge with a new node which doesn't write to any registers and has
   both of its outgoing edges pointing to the target of the original edge.
 - Inverse of above.
 - Replace a node pointed to from both of its parent's edges by two identical
   nodes, both of which point to the same child nodes.
 - Inverse of above.

Behavior changing mutations:
 - Redirecting an edge to a different node.
 - Changing a term.
 - Changing a predicate.

Can we propagate forbidden outputs as forbidden register states up the tree?
Probably, in some form.

As long as there's no local maxima, should we greedily take each small
improvement we can find? Maybe.

I think the above transformations guarantee no local maxima, since a new node
can always be inserted at the root, havea series of constant matches to
distinguish a specific input, then produce the right output.

Two of these arms can arise, for different inputs. If they "can be combined"
then the bottom nodes can become identical, and can be merged. Then, the two
arms can be "zipped" together.

Obviously, that's not a proof. But maybe one can actually be done by induction
on the number of examples? The base case looks easy to prove. The main problem
is that defining precisely "can be combined" is very hard.

There's also the problem that it might take arbitrarily long to escape local
maxima, or to stabilize on a "good" or small solution for the maxima.

It would be convenient to have a textual representation of rule diagrams.

Maybe something like this:

```
  match root x($0 <- $1, _, #10) { out } { }
  output out y($1, $2)
```

Let's try to write Tic-Tac-Toe as a rule diagram:

```
  root: player($player <- _) { check_move }
  check_move: move($x <- _, $y <- _) {
    board($x, $y, :blank) {
      do_move
    }
  }
  do_move: board($x <- $x, $y <- $y, $p <- _) {
    output next_board($x, $y, $player)
  } {
    output next_board($x, $y, $p)
  }
```

Doing that easily required changing the meaning of reject. Instead of a reject
path being activated only if no facts matched the node, the reject path is
activated for each fact which doesn't match the node.

This is usually more powerful. It also makes indicies useless for these nodes.
For the current evaluation method, there's no indication that indicies are even
a good idea, since the number of input facts is very small.


```
  root: player(%0 <- _) { check_move }
  check_move: move(%1 <- _, %2 <- _) { check_space }
  check_space: board(%1, %2, :blank) { do_move }
  do_move: board(%1 <- %1, %2 <- %2, %3 <- _) {
    output next_board(%1, %2, %0)
  } {
    output next_board(%1, %2, %3)
  }
```

It actually works better with the arrows going the other way.


The current organization of everything except the diagrams themselves are
terrible. The predicate table gets copied around a bunch of places. We should
draw up generally how information flows.
Algorithm overview:
  1. We get a problem statement.
  2. We extract the space of input and output facts from the problem.
  3. We seed the evolutionary algorithm
  4. Each individual in the population is initially evaluated.
  5. In a loop, we do the following:
    1. We run some analysis to bound the mutation space.
    2. Mutations are applied to produce a larger population.
    3. The larger population is evaluated.
    4. The best evaluated individuals are kept.
