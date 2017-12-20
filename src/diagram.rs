#![allow(unused_imports)]
use std::collections::HashSet;
use std::iter;

use database::Database;
use fact::Fact;
use fixgraph::{EdgeIndex, FixGraph, NodeIndex};
use predicate::Predicate;
use registers::{RegisterFile, RegisterSet};
use simple_query::{SimpleQuery, SimpleQueryTerm};
use value::Value;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct MatchTerm {
    constraint: MatchTermConstraint,
    target: Option<usize>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum MatchTermConstraint {
    Register(usize),
    Constant(Value),
    Free,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum OutputTerm {
    Register(usize),
    Constant(Value),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum Node {
    Match {
        predicate: Predicate,
        terms: Vec<MatchTerm>,
    },
    Output {
        predicate: Predicate,
        terms: Vec<OutputTerm>,
    },
}

enum PropagateOutput {
    Registers(RegisterSet, RegisterSet),
    Database(Database),
}

#[derive(Clone, Debug)]
struct Diagram {
    num_registers: usize,
    graph: FixGraph<Node>,
}

impl Diagram {
    pub fn new(num_registers: usize) -> Self {
        Diagram {
            num_registers,
            graph: FixGraph::new(2),
        }
    }

    fn propagate(
        &self,
        node: NodeIndex,
        database: &Database,
        registers: &RegisterSet,
    ) -> PropagateOutput {
        match *self.graph.get_node(node) {
            Node::Match {
                predicate,
                ref terms,
            } => {
                let mut match_set = RegisterSet::new(registers.num_registers());
                let mut refute_set = RegisterSet::new(registers.num_registers());
                let mut query_terms = Vec::with_capacity(terms.len());
                for register_file in registers.iter() {
                    for term in terms {
                        query_terms.push(match &term.constraint {
                            &MatchTermConstraint::Free => SimpleQueryTerm::Free,
                            &MatchTermConstraint::Constant(ref value) => {
                                SimpleQueryTerm::Constant { value }
                            }
                            &MatchTermConstraint::Register(index) => {
                                if index >= register_file.len() {
                                    SimpleQueryTerm::Constant { value: &Value::Nil }
                                } else {
                                    if let Some(ref value) = register_file[index] {
                                        SimpleQueryTerm::Constant { value }
                                    } else {
                                        SimpleQueryTerm::Free
                                    }
                                }
                            }
                        });
                    }
                    let mut query_iter = database
                        .simple_query(SimpleQuery {
                            predicate,
                            terms: &query_terms,
                        })
                        .peekable();
                    if query_iter.peek().is_some() {
                        for fact in query_iter {
                            let mut r = register_file.clone();
                            for (term, value) in terms.iter().zip(fact.values.iter()) {
                                if let Some(target) = term.target {
                                    if target < r.len() {
                                        r[target] = Some(value.clone());
                                    }
                                };
                            }
                            match_set.push(r);
                        }
                    } else {
                        refute_set.push(register_file.clone());
                    }
                    query_terms.clear();
                }
                PropagateOutput::Registers(match_set, refute_set)
            }
            Node::Output {
                predicate,
                ref terms,
            } => {
                let mut result_db = Database::new();
                for register_file in registers.iter() {
                    let mut values = Vec::with_capacity(terms.len());
                    for term in terms {
                        match *term {
                            OutputTerm::Constant(ref value) => {
                                values.push(value.clone());
                            }
                            OutputTerm::Register(index) => {
                                if index < register_file.len() {
                                    if let Some(ref value) = register_file[index] {
                                        values.push(value.clone());
                                    } else {
                                        values.push(Value::Nil);
                                    }
                                }
                            }
                        }
                    }
                    result_db.insert_fact(Fact {
                        predicate,
                        values: &values[..],
                    });
                }
                PropagateOutput::Database(result_db)
            }
        }
    }

    pub fn insert_node(&mut self, node: Node) -> NodeIndex {
        self.graph.push(node)
    }

    pub fn get_node(&self, index: NodeIndex) -> &Node {
        self.graph.get_node(index)
    }

    pub fn get_node_mut(&mut self, index: NodeIndex) -> &mut Node {
        self.graph.get_node_mut(index)
    }

    pub fn set_on_match(&mut self, src: NodeIndex, target: NodeIndex) {
        self.graph.set_edge_target(src, EdgeIndex(1), Some(target));
    }

    pub fn set_on_refute(&mut self, src: NodeIndex, target: NodeIndex) {
        self.graph.set_edge_target(src, EdgeIndex(0), Some(target));
    }

    pub fn clear_on_match(&mut self, src: NodeIndex) {
        self.graph.set_edge_target(src, EdgeIndex(1), None);
    }

    pub fn clear_on_refute(&mut self, src: NodeIndex) {
        self.graph.set_edge_target(src, EdgeIndex(0), None);
    }

    pub fn get_on_match(&self, src: NodeIndex) -> Option<NodeIndex> {
        self.graph.get_edge_target(src, EdgeIndex(1))
    }

    pub fn get_on_refute(&self, src: NodeIndex) -> Option<NodeIndex> {
        self.graph.get_edge_target(src, EdgeIndex(0))
    }

    pub fn len(&self) -> usize {
        self.graph.len()
    }

    pub fn evaluate(&self, input: &Database) -> Database {
        Evaluation::run(self, input, self.num_registers).total_db
    }
}

#[derive(Clone, Debug)]
pub struct Evaluation {
    input_sets: Vec<RegisterSet>,
    output_sets: Vec<(RegisterSet, RegisterSet)>,
    output_dbs: Vec<Option<Database>>,
    total_db: Database,
}

impl Evaluation {
    fn run(diagram: &Diagram, input: &Database, num_registers: usize) -> Self {
        let mut input_sets: Vec<RegisterSet> = iter::repeat(RegisterSet::new(num_registers))
            .take(diagram.len())
            .collect();
        let mut output_sets: Vec<(RegisterSet, RegisterSet)> = iter::repeat((
            RegisterSet::new(num_registers),
            RegisterSet::new(num_registers),
        )).take(diagram.len())
            .collect();
        input_sets[0].push(RegisterFile::new(num_registers));
        let mut output_dbs: Vec<_> = iter::repeat(None).take(diagram.len()).collect();
        let mut pending_nodes = vec![NodeIndex(0)];
        while let Some(node_index) = pending_nodes.pop() {
            match diagram.propagate(node_index, input, &input_sets[node_index.0]) {
                PropagateOutput::Registers(match_set, refute_set) => {
                    let (ref mut old_match_set, ref mut old_refute_set) = output_sets[node_index.0];
                    if *old_match_set != match_set {
                        for registers in match_set.iter() {
                            old_match_set.push(registers.clone());
                        }
                        if let Some(match_node) = diagram.get_on_match(node_index) {
                            let input_sets = &mut input_sets[match_node.0];
                            for registers in match_set.iter() {
                                input_sets.push(registers.clone());
                            }
                            if !pending_nodes.contains(&match_node) {
                                pending_nodes.push(match_node);
                            }
                        }
                    }
                    if *old_refute_set != refute_set {
                        for registers in refute_set.iter() {
                            old_refute_set.push(registers.clone());
                        }
                        if let Some(refute_node) = diagram.get_on_refute(node_index) {
                            let input_sets = &mut input_sets[refute_node.0];
                            for registers in refute_set.iter() {
                                input_sets.push(registers.clone());
                            }
                            if !pending_nodes.contains(&refute_node) {
                                pending_nodes.push(refute_node);
                            }
                        }
                    }
                }
                PropagateOutput::Database(db) => {
                    output_dbs[node_index.0] = Some(db);
                }
            }
        }
        let mut total_db = Database::new();
        for db in output_dbs.iter().filter_map(|db| db.as_ref()) {
            for fact in db.all_facts() {
                total_db.insert_fact(fact);
            }
        }
        Evaluation {
            input_sets,
            output_sets,
            output_dbs,
            total_db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_evaluate_constant_diagram() {
        let mut diagram = Diagram::new(0);
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
        let mut diagram = Diagram::new(2);
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
        let mut diagram = Diagram::new(2);
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
        let mut diagram = Diagram::new(2);
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
