use fixgraph::NodeIndex;
use predicate::Predicate;
use value::Value;

/*
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
 */

#[derive(Clone, Copy, Debug)]
pub enum Edge {
    Root,
    Match(NodeIndex),
    Refute(NodeIndex),
}

#[derive(Clone, Copy, Debug)]
pub struct Term(pub NodeIndex, pub usize);

#[derive(Clone, Debug)]
pub enum Mutation {
    SetConstraintRegister {
        term: Term,
        register: usize,
    },
    SetConstraintConstant {
        term: Term,
        value: Value,
    },
    SetConstraintFree {
        term: Term,
    },
    SetTarget {
        term: Term,
        register: Option<usize>,
    },
    InsertPassthrough {
        predicate: Predicate,
        num_terms: usize,
        edge: Edge,
    },
    RemoveNode {
        node: NodeIndex,
    },
    SetEdge {
        edge: Edge,
        target: NodeIndex,
    },
    SetOutputRegister {
        term: Term,
        register: usize,
    },
    SetOutputConstant {
        term: Term,
        value: Value,
    },
    SetPredicate {
        node: NodeIndex,
        predicate: Predicate,
    },
}
