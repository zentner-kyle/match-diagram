use std::fmt;

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

impl EdgeGroup {
    pub fn edge_to(self, target: NodeIndex) -> Edge {
        match self {
            EdgeGroup::Roots => Edge::Root(target),
            EdgeGroup::MatchTargets(source) => Edge::Match { source, target },
            EdgeGroup::RefuteTargets(source) => Edge::Match { source, target },
            _ => panic!("can only make an edge to target given a source group"),
        }
    }
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

impl Edge {
    pub fn source(self) -> Option<NodeIndex> {
        match self {
            Edge::Root(_) => None,
            Edge::Match { source, .. } | Edge::Refute { source, .. } => Some(source),
        }
    }

    pub fn target(self) -> NodeIndex {
        match self {
            Edge::Root(target) => target,
            Edge::Match { target, .. } | Edge::Refute { target, .. } => target,
        }
    }

    pub fn nodes(self) -> MaybeNodePair {
        match self {
            Edge::Root(node) => MaybeNodePair::One(node),
            Edge::Match { source, target } | Edge::Refute { source, target } => {
                MaybeNodePair::Two(source, target)
            }
        }
    }

    pub fn forward_group(self) -> EdgeGroup {
        match self {
            Edge::Root(_) => EdgeGroup::Roots,
            Edge::Match { source, .. } => EdgeGroup::MatchTargets(source),
            Edge::Refute { source, .. } => EdgeGroup::RefuteTargets(source),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum MaybeNodePair {
    Zero,
    One(NodeIndex),
    Two(NodeIndex, NodeIndex),
}

impl Iterator for MaybeNodePair {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<NodeIndex> {
        match *self {
            MaybeNodePair::Zero => None,
            MaybeNodePair::One(node) => {
                *self = MaybeNodePair::Zero;
                Some(node)
            }
            MaybeNodePair::Two(first, second) => {
                *self = MaybeNodePair::One(second);
                Some(first)
            }
        }
    }
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

impl Node {
    pub fn is_match(&self) -> bool {
        if let &Node::Match { .. } = self {
            true
        } else {
            false
        }
    }
}

pub trait MultiDiagram: fmt::Debug {
    fn insert_node(&mut self, node: Node) -> NodeIndex;

    fn get_node(&self, index: NodeIndex) -> &Node;

    fn get_node_mut(&mut self, index: NodeIndex) -> &mut Node;

    fn get_group(&self, group: EdgeGroup) -> &[NodeIndex];

    fn edge_exists(&self, edge: Edge) -> bool;

    fn insert_edge(&mut self, edge: Edge);

    fn remove_edge(&mut self, edge: Edge);

    fn len(&self) -> usize;

    fn insert_edge_if_not_present(&mut self, edge: Edge) -> bool {
        if self.edge_exists(edge) {
            true
        } else {
            self.insert_edge(edge);
            false
        }
    }

    fn remove_edge_if_present(&mut self, edge: Edge) -> bool {
        if self.edge_exists(edge) {
            self.remove_edge(edge);
            true
        } else {
            false
        }
    }
}

pub trait Diagram: MultiDiagram {
    fn get_root(&self) -> NodeIndex;

    fn set_root(&mut self, root: NodeIndex);

    fn set_on_match(&mut self, src: NodeIndex, target: NodeIndex);

    fn set_on_refute(&mut self, src: NodeIndex, target: NodeIndex);

    fn clear_on_match(&mut self, src: NodeIndex);

    fn clear_on_refute(&mut self, src: NodeIndex);

    fn get_on_match(&self, src: NodeIndex) -> Option<NodeIndex>;

    fn get_on_refute(&self, src: NodeIndex) -> Option<NodeIndex>;

    fn get_match_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]>;

    fn get_refute_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]>;

    fn get_num_registers(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct DiagramSpace {
    pub num_nodes: usize,
    pub num_registers: usize,
    pub num_terms: usize,
}
