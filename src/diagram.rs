use database::Database;
use fact::Fact;
use fixgraph::{EdgeIndex, FixGraph, NodeIndex};
use predicate::Predicate;
use simple_query::{SimpleQuery, SimpleQueryTerm};
use value::Value;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum MatchTerm {
    Up { distance: usize, term: usize },
    Constant { value: Value },
    Free,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum OutputTerm {
    Up { distance: usize, term: usize },
    Constant { value: Value },
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

#[derive(Clone, Debug)]
struct Diagram {
    graph: FixGraph<Node>,
}

impl Diagram {
    pub fn new() -> Self {
        Diagram {
            graph: FixGraph::new(2),
        }
    }

    pub fn evaluate(&self, database: &Database) -> Database {
        let mut fact_stack = Vec::new();
        self.evaluate_node(NodeIndex(0), database, &mut fact_stack)
    }

    pub fn evaluate_node<'a, 'b, 'c>(
        &'a self,
        node: NodeIndex,
        database: &'b Database,
        fact_stack: &'c mut Vec<Fact<'b>>,
    ) -> Database {
        let mut result = Database::new();
        match self.graph.get_node(node) {
            &Node::Match {
                predicate,
                ref terms,
            } => {
                let mut query_terms = Vec::with_capacity(terms.len());
                for term in terms {
                    query_terms.push(match term {
                        &MatchTerm::Free => SimpleQueryTerm::Free,
                        &MatchTerm::Constant { ref value } => SimpleQueryTerm::Constant { value },
                        &MatchTerm::Up { distance, term } => {
                            if distance > fact_stack.len() {
                                return result;
                            }
                            SimpleQueryTerm::Constant {
                                value: &fact_stack[fact_stack.len() - distance].values[term],
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
                    if let Some(next_node) = self.get_on_match(node) {
                        for fact in query_iter {
                            fact_stack.push(fact);
                            let sub_result = self.evaluate_node(next_node, database, fact_stack);
                            fact_stack.pop();
                            for fact in sub_result.all_facts() {
                                result.insert_fact(fact);
                            }
                        }
                    };
                } else {
                    if let Some(next_node) = self.get_on_refute(node) {
                        let sub_result = self.evaluate_node(next_node, database, fact_stack);
                        for fact in sub_result.all_facts() {
                            result.insert_fact(fact);
                        }
                    };
                }
            }
            &Node::Output {
                predicate,
                ref terms,
            } => {
                let mut values = Vec::with_capacity(terms.len());
                for term in terms {
                    values.push(match term {
                        &OutputTerm::Constant { ref value } => value.clone(),
                        &OutputTerm::Up { distance, term } => {
                            if distance > fact_stack.len() {
                                return result;
                            }
                            fact_stack[fact_stack.len() - distance].values[term].clone()
                        }
                    });
                }
                result.insert_fact(Fact {
                    predicate,
                    values: &values,
                });
            }
        }
        return result;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_evaluate_constant_diagram() {
        let mut diagram = Diagram::new();
        let output_node = Node::Output {
            predicate: Predicate(0),
            terms: vec![
                OutputTerm::Constant {
                    value: Value::Symbol(1),
                },
                OutputTerm::Constant {
                    value: Value::Symbol(2),
                },
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
        let mut diagram = Diagram::new();
        let match_anything_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![MatchTerm::Free, MatchTerm::Free],
        };
        let output_node = Node::Output {
            predicate: Predicate(1),
            terms: vec![
                OutputTerm::Up {
                    distance: 1,
                    term: 0,
                },
                OutputTerm::Up {
                    distance: 1,
                    term: 1,
                },
            ],
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
        let mut diagram = Diagram::new();
        let match_ones_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm::Constant {
                    value: Value::Symbol(1),
                },
                MatchTerm::Free,
            ],
        };
        let output_node = Node::Output {
            predicate: Predicate(1),
            terms: vec![
                OutputTerm::Up {
                    distance: 1,
                    term: 0,
                },
                OutputTerm::Up {
                    distance: 1,
                    term: 1,
                },
            ],
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
        let mut result_facts = result_database.all_facts();
        assert_eq!(
            result_facts.next(),
            Some(Fact {
                predicate: Predicate(1),
                values: &[Value::Symbol(1), Value::Symbol(2),],
            })
        );
        assert_eq!(
            result_facts.next(),
            Some(Fact {
                predicate: Predicate(1),
                values: &[Value::Symbol(1), Value::Symbol(3),],
            })
        );
        assert_eq!(result_facts.next(), None);
        assert_eq!(result_facts.next(), None);
    }

    #[test]
    fn can_evaluate_nested_filtering_diagram() {
        let mut diagram = Diagram::new();
        let match_ones_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm::Constant {
                    value: Value::Symbol(1),
                },
                MatchTerm::Free,
            ],
        };
        let match_anything_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![MatchTerm::Free, MatchTerm::Free],
        };
        let output_node = Node::Output {
            predicate: Predicate(1),
            terms: vec![
                OutputTerm::Up {
                    distance: 2,
                    term: 0,
                },
                OutputTerm::Up {
                    distance: 1,
                    term: 1,
                },
            ],
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
        let mut result_facts = result_database.all_facts();
        for _ in 0..2 {
            assert_eq!(
                result_facts.next(),
                Some(Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(2),],
                })
            );
            assert_eq!(
                result_facts.next(),
                Some(Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(3),],
                })
            );
            assert_eq!(
                result_facts.next(),
                Some(Fact {
                    predicate: Predicate(1),
                    values: &[Value::Symbol(1), Value::Symbol(4),],
                })
            );
        }
        assert_eq!(result_facts.next(), None);
        assert_eq!(result_facts.next(), None);
    }
}
