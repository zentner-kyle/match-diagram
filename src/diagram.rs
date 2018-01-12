use database::Database;
use node_index::NodeIndex;
use predicate::Predicate;
use registers::RegisterSet;
use value::Value;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum EdgeGroup {
    Roots,
    MatchTargets(NodeIndex),
    RefuteTargets(NodeIndex),
    MatchSources(NodeIndex),
    RefuteSources(NodeIndex),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Edge {
    Root(NodeIndex),
    Match {
        source: NodeIndex,
        target: NodeIndex,
    },
    Refute {
        source: NodeIndex,
        target: NodeIndex,
    },
}

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

pub trait MultiDiagram {
    fn insert_node2(&mut self, node: Node) -> NodeIndex;

    fn get_node(&self, index: NodeIndex) -> &Node;

    fn get_node_mut(&mut self, index: NodeIndex) -> &mut Node;

    fn get_group(&self, group: EdgeGroup) -> &[NodeIndex];

    fn edge_exists(&self, edge: Edge) -> bool;

    fn insert_edge(&mut self, edge: Edge);

    fn remove_edge(&mut self, edge: Edge);

    fn len2(&self) -> usize;
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
