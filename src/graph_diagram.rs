use std::collections::{hash_map, HashMap};

use database::Database;
use diagram::{Diagram, Node};
use evaluation::Evaluation;
use fixgraph::{EdgeIndex, FixGraph};
use node_index::NodeIndex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Edges {
    on_match: Vec<NodeIndex>,
    on_refute: Vec<NodeIndex>,
}

impl Edges {
    fn new() -> Self {
        Edges {
            on_match: Vec::new(),
            on_refute: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GraphNode {
    node: Node,
    out_edges: Edges,
    in_edges: Edges,
}

impl GraphNode {
    fn new(node: Node) -> Self {
        GraphNode {
            node,
            out_edges: Edges::new(),
            in_edges: Edges::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphDiagram {
    num_registers: usize,
    root: Vec<NodeIndex>,
    graph: Vec<GraphNode>,
}

impl GraphDiagram {
    pub fn new(num_registers: usize) -> Self {
        GraphDiagram {
            num_registers,
            root: Vec::new(),
            graph: Vec::new(),
        }
    }

    pub fn evaluate(&self, input: &Database) -> Database {
        Evaluation::run(self, input).total_db
    }

    pub fn match_source_group(&self, node: NodeIndex) -> &Vec<NodeIndex> {
        &self.graph[node.0].in_edges.on_match
    }

    pub fn refute_source_group(&self, node: NodeIndex) -> &Vec<NodeIndex> {
        &self.graph[node.0].in_edges.on_refute
    }

    pub fn match_source_group_mut(&mut self, node: NodeIndex) -> &mut Vec<NodeIndex> {
        &mut self.graph[node.0].in_edges.on_match
    }

    pub fn refute_source_group_mut(&mut self, node: NodeIndex) -> &mut Vec<NodeIndex> {
        &mut self.graph[node.0].in_edges.on_refute
    }

    pub fn match_target_group(&self, node: NodeIndex) -> &Vec<NodeIndex> {
        &self.graph[node.0].out_edges.on_match
    }

    pub fn refute_target_group(&self, node: NodeIndex) -> &Vec<NodeIndex> {
        &self.graph[node.0].out_edges.on_refute
    }

    pub fn match_target_group_mut(&mut self, node: NodeIndex) -> &mut Vec<NodeIndex> {
        &mut self.graph[node.0].out_edges.on_match
    }

    pub fn refute_target_group_mut(&mut self, node: NodeIndex) -> &mut Vec<NodeIndex> {
        &mut self.graph[node.0].out_edges.on_refute
    }
}

fn remove_from_group(group: &mut Vec<NodeIndex>, node: NodeIndex) {
    let position = group
        .iter()
        .position(|n| *n == node)
        .expect("Should only remove a node if it is present in a group");
    group.swap_remove(position);
}

fn insert_into_group(group: &mut Vec<NodeIndex>, node: NodeIndex) {
    if group.iter().any(|n| *n == node) {
        panic!("Should only insert a node if it is not present in a group");
    }
    group.push(node);
}

impl Diagram for GraphDiagram {
    fn get_root(&self) -> NodeIndex {
        self.root[0]
    }

    fn set_root(&mut self, root: NodeIndex) {
        self.root.clear();
        self.root.push(root);
    }

    fn insert_node(&mut self, node: Node) -> NodeIndex {
        let result = NodeIndex(self.len());
        self.graph.push(GraphNode::new(node));
        result
    }

    fn get_node(&self, index: NodeIndex) -> &Node {
        &self.graph[index.0].node
    }

    fn get_node_mut(&mut self, index: NodeIndex) -> &mut Node {
        &mut self.graph[index.0].node
    }

    fn set_on_match(&mut self, src: NodeIndex, target: NodeIndex) {
        if let Some(target) = self.get_on_match(src) {
            remove_from_group(self.match_source_group_mut(target), src);
        }
        {
            let edges = self.match_target_group_mut(src);
            edges.clear();
            edges.push(target);
        }
        insert_into_group(self.match_source_group_mut(target), src);
    }

    fn set_on_refute(&mut self, src: NodeIndex, target: NodeIndex) {
        if let Some(target) = self.get_on_refute(src) {
            remove_from_group(self.refute_source_group_mut(target), src);
        }
        {
            let edges = self.refute_target_group_mut(src);
            edges.clear();
            edges.push(target);
        }
        insert_into_group(self.refute_source_group_mut(target), src);
    }

    fn clear_on_match(&mut self, src: NodeIndex) {
        if let Some(target) = self.get_on_match(src) {
            remove_from_group(self.match_source_group_mut(target), src);
        }
        self.match_target_group_mut(src).clear();
    }

    fn clear_on_refute(&mut self, src: NodeIndex) {
        if let Some(target) = self.get_on_refute(src) {
            remove_from_group(self.refute_source_group_mut(target), src);
        }
        self.refute_target_group_mut(src).clear();
    }

    fn get_on_match(&self, src: NodeIndex) -> Option<NodeIndex> {
        self.match_target_group(src).get(0).cloned()
    }

    fn get_on_refute(&self, src: NodeIndex) -> Option<NodeIndex> {
        self.refute_target_group(src).get(0).cloned()
    }

    fn len(&self) -> usize {
        self.graph.len()
    }

    fn get_match_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]> {
        Some(self.match_source_group(target).as_ref())
    }

    fn get_refute_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]> {
        Some(self.refute_source_group(target).as_ref())
    }

    fn get_num_registers(&self) -> usize {
        self.num_registers
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use diagram::{MatchTerm, MatchTermConstraint, OutputTerm};
    use fact::Fact;
    use predicate::Predicate;
    use value::Value;

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
        let root = diagram.insert_node(output_node);
        diagram.set_root(root);
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
        diagram.set_root(root);
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
        diagram.set_root(root);
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
        diagram.set_root(root);
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
