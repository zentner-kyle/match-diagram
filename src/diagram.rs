use fixgraph::FixGraph;
use predicate::Predicate;
use value::Value;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum Term {
    Up { distance: usize, term: usize },
    Constant { value: Value },
    Free,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct MatchNode {
    predicate: Predicate,
    terms: Vec<Term>,
}

#[derive(Clone, Debug)]
struct Diagram {
    graph: FixGraph<MatchNode>,
}
