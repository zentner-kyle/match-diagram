use database::Database;
use fixgraph::NodeIndex;
use predicate::Predicate;
use registers::RegisterSet;
use value::Value;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MatchTerm {
    pub constraint: MatchTermConstraint,
    pub target: Option<usize>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum MatchTermConstraint {
    Register(usize),
    Constant(Value),
    Free,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum OutputTerm {
    Register(usize),
    Constant(Value),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Node {
    Match {
        predicate: Predicate,
        terms: Vec<MatchTerm>,
    },
    Output {
        predicate: Predicate,
        terms: Vec<OutputTerm>,
    },
}

pub trait Diagram {
    fn get_root(&self) -> NodeIndex;

    fn set_root(&mut self, root: NodeIndex);

    fn insert_node(&mut self, node: Node) -> NodeIndex;

    fn get_node(&self, index: NodeIndex) -> &Node;

    fn get_node_mut(&mut self, index: NodeIndex) -> &mut Node;

    fn set_on_match(&mut self, src: NodeIndex, target: NodeIndex);

    fn set_on_refute(&mut self, src: NodeIndex, target: NodeIndex);

    fn clear_on_match(&mut self, src: NodeIndex);

    fn clear_on_refute(&mut self, src: NodeIndex);

    fn get_on_match(&self, src: NodeIndex) -> Option<NodeIndex>;

    fn get_on_refute(&self, src: NodeIndex) -> Option<NodeIndex>;

    fn len(&self) -> usize;

    fn get_match_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]>;

    fn get_refute_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]>;

    fn get_num_registers(&self) -> usize;
}
