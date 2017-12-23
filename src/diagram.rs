#![allow(unused_imports)]
use std::collections::{hash_map, HashMap, HashSet};
use std::iter;

use database::Database;
use evaluation::Evaluation;
use fact::Fact;
use fixgraph::{EdgeIndex, FixGraph, NodeIndex};
use predicate::Predicate;
use registers::{RegisterFile, RegisterSet};
use simple_query::{SimpleQuery, SimpleQueryTerm};
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

pub enum PropagateOutput {
    Registers(RegisterSet, RegisterSet),
    Database(Database),
}

pub trait Diagram {
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

    fn get_match_sources(&self, child: NodeIndex) -> Option<&[NodeIndex]>;

    fn get_refute_sources(&self, child: NodeIndex) -> Option<&[NodeIndex]>;
}

#[derive(Clone, Debug)]
pub struct GraphDiagram {
    num_registers: usize,
    graph: FixGraph<Node>,
    match_sources: HashMap<NodeIndex, Vec<NodeIndex>>,
    refute_sources: HashMap<NodeIndex, Vec<NodeIndex>>,
}

impl GraphDiagram {
    pub fn new(num_registers: usize) -> Self {
        GraphDiagram {
            num_registers,
            graph: FixGraph::new(2),
            match_sources: HashMap::new(),
            refute_sources: HashMap::new(),
        }
    }

    pub fn evaluate(&self, input: &Database) -> Database {
        Evaluation::run(self, input, self.num_registers).total_db
    }
}

fn insert_source(
    sources: &mut HashMap<NodeIndex, Vec<NodeIndex>>,
    src: NodeIndex,
    target: NodeIndex,
) {
    match sources.entry(target) {
        hash_map::Entry::Occupied(mut entry) => {
            if !entry.get().contains(&src) {
                entry.get_mut().push(src);
            }
        }
        hash_map::Entry::Vacant(entry) => {
            entry.insert(vec![src]);
        }
    }
}

fn remove_source(
    sources: &mut HashMap<NodeIndex, Vec<NodeIndex>>,
    src: NodeIndex,
    target: NodeIndex,
) {
    let sources = sources
        .get_mut(&target)
        .expect("Should only be removing source which exists");
    let index = sources
        .iter()
        .position(|&s| s == src)
        .expect("src should be present in the sources of target");
    sources.remove(index);
}

impl Diagram for GraphDiagram {
    fn insert_node(&mut self, node: Node) -> NodeIndex {
        self.graph.push(node)
    }

    fn get_node(&self, index: NodeIndex) -> &Node {
        self.graph.get_node(index)
    }

    fn get_node_mut(&mut self, index: NodeIndex) -> &mut Node {
        self.graph.get_node_mut(index)
    }

    fn set_on_match(&mut self, src: NodeIndex, target: NodeIndex) {
        self.graph.set_edge_target(src, EdgeIndex(1), Some(target));
        insert_source(&mut self.match_sources, src, target);
    }

    fn set_on_refute(&mut self, src: NodeIndex, target: NodeIndex) {
        self.graph.set_edge_target(src, EdgeIndex(0), Some(target));
        insert_source(&mut self.refute_sources, src, target);
    }

    fn clear_on_match(&mut self, src: NodeIndex) {
        if let Some(target) = self.get_on_match(src) {
            remove_source(&mut self.match_sources, src, target);
        }
        self.graph.set_edge_target(src, EdgeIndex(1), None);
    }

    fn clear_on_refute(&mut self, src: NodeIndex) {
        if let Some(target) = self.get_on_refute(src) {
            remove_source(&mut self.refute_sources, src, target);
        }
        self.graph.set_edge_target(src, EdgeIndex(0), None);
    }

    fn get_on_match(&self, src: NodeIndex) -> Option<NodeIndex> {
        self.graph.get_edge_target(src, EdgeIndex(1))
    }

    fn get_on_refute(&self, src: NodeIndex) -> Option<NodeIndex> {
        self.graph.get_edge_target(src, EdgeIndex(0))
    }

    fn len(&self) -> usize {
        self.graph.len()
    }

    fn get_match_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]> {
        self.match_sources.get(&target).map(|v| &v[..])
    }

    fn get_refute_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]> {
        self.refute_sources.get(&target).map(|v| &v[..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_evaluate_constant_diagram() {
        let mut diagram = GraphDiagram::new(0);
        let output_node = Node::Output {
            predicate: Predicate(0),
            terms: vec![
                OutputTerm::Constant(Value::Symbol(1)),
                OutputTerm::Constant(Value::Symbol(2)),
            ],
        };
        diagram.insert_node(output_node);
        let database = Database::new();
        let result_database = diagram.evaluate(&database);
        let mut result_facts = result_database.all_facts();
        assert_eq!(
            result_facts.next(),
            Some(Fact {
                predicate: Predicate(0),
                values: &[Value::Symbol(1), Value::Symbol(2),],
            })
        );
        assert_eq!(result_facts.next(), None);
        assert_eq!(result_facts.next(), None);
    }

    #[test]
    fn can_evaluate_copying_diagram() {
        let mut diagram = GraphDiagram::new(2);
        let match_anything_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(0),
                },
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
            ],
        };
        let output_node = Node::Output {
            predicate: Predicate(1),
            terms: vec![OutputTerm::Register(0), OutputTerm::Register(1)],
        };
        let root = diagram.insert_node(match_anything_node);
        assert_eq!(root, NodeIndex(0));
        let output = diagram.insert_node(output_node);
        diagram.set_on_match(root, output);
        let mut database = Database::new();
        let input_fact = Fact {
            predicate: Predicate(0),
            values: &[Value::Symbol(1), Value::Symbol(2)],
        };
        database.insert_fact(input_fact);
        let result_database = diagram.evaluate(&database);
        let mut result_facts = result_database.all_facts();
        assert_eq!(
            result_facts.next(),
            Some(Fact {
                predicate: Predicate(1),
                values: &[Value::Symbol(1), Value::Symbol(2),],
            })
        );
        assert_eq!(result_facts.next(), None);
        assert_eq!(result_facts.next(), None);
    }

    #[test]
    fn can_evaluate_filtering_diagram() {
        let mut diagram = GraphDiagram::new(2);
        let match_ones_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm {
                    constraint: MatchTermConstraint::Constant(Value::Symbol(1)),
                    target: Some(0),
                },
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
            ],
        };
        let output_node = Node::Output {
            predicate: Predicate(1),
            terms: vec![OutputTerm::Register(0), OutputTerm::Register(1)],
        };
        let root = diagram.insert_node(match_ones_node);
        assert_eq!(root, NodeIndex(0));
        let output = diagram.insert_node(output_node);
        diagram.set_on_match(root, output);
        let mut database = Database::new();
        let input_facts = [
            Fact {
                predicate: Predicate(0),
                values: &[Value::Symbol(1), Value::Symbol(2)],
            },
            Fact {
                predicate: Predicate(0),
                values: &[Value::Symbol(2), Value::Symbol(3)],
            },
            Fact {
                predicate: Predicate(0),
                values: &[Value::Symbol(1), Value::Symbol(3)],
            },
        ];
        for input_fact in input_facts.iter().cloned() {
            database.insert_fact(input_fact);
        }
        let result_database = diagram.evaluate(&database);
        let result_facts: HashSet<_> = result_database.all_facts().collect();
        assert_eq!(
            result_facts,
            [
                Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(2),],
                },
                Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(3),],
                }
            ].iter()
                .cloned()
                .collect()
        );
    }

    #[test]
    fn can_evaluate_nested_filtering_diagram() {
        let mut diagram = GraphDiagram::new(2);
        let match_ones_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm {
                    constraint: MatchTermConstraint::Constant(Value::Symbol(1)),
                    target: Some(0),
                },
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
            ],
        };
        let match_anything_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: None,
                },
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
            ],
        };
        let output_node = Node::Output {
            predicate: Predicate(1),
            terms: vec![OutputTerm::Register(0), OutputTerm::Register(1)],
        };
        let root = diagram.insert_node(match_ones_node);
        let anything = diagram.insert_node(match_anything_node);
        let output = diagram.insert_node(output_node);
        diagram.set_on_match(root, anything);
        diagram.set_on_match(anything, output);
        let mut database = Database::new();
        let input_facts = [
            Fact {
                predicate: Predicate(0),
                values: &[Value::Symbol(1), Value::Symbol(2)],
            },
            Fact {
                predicate: Predicate(0),
                values: &[Value::Symbol(2), Value::Symbol(3)],
            },
            Fact {
                predicate: Predicate(0),
                values: &[Value::Symbol(1), Value::Symbol(4)],
            },
        ];
        for input_fact in input_facts.iter().cloned() {
            database.insert_fact(input_fact);
        }
        let result_database = diagram.evaluate(&database);
        let result_facts: HashSet<_> = result_database.all_facts().collect();
        assert_eq!(
            result_facts,
            [
                Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(2),],
                },
                Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(4),],
                },
                Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(3),],
                }
            ].iter()
                .cloned()
                .collect()
        );
    }
}
