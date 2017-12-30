use diagram::{Diagram, MatchTerm, MatchTermConstraint, Node, OutputTerm};
use fixgraph::NodeIndex;
use mutation::{Edge, Mutation, Term};
use std::iter;

pub struct MutationResult {
    phenotype_could_have_changed: bool,
    node_to_restart: Option<NodeIndex>,
}

fn changed_node(node: NodeIndex) -> Option<MutationResult> {
    Some(MutationResult {
        phenotype_could_have_changed: true,
        node_to_restart: Some(node),
    })
}

pub fn apply_mutation<D: Diagram>(diagram: &mut D, mutation: Mutation) -> Option<MutationResult> {
    match mutation {
        Mutation::SetConstraintRegister {
            term: Term(node, term),
            register,
        } => {
            if let &mut Node::Match { ref mut terms, .. } = diagram.get_node_mut(node) {
                if term < terms.len() {
                    terms[term].constraint = MatchTermConstraint::Register(register);
                    return changed_node(node);
                }
            };
            return None;
        }
        Mutation::SetConstraintConstant {
            term: Term(node, term),
            value,
        } => {
            if let &mut Node::Match { ref mut terms, .. } = diagram.get_node_mut(node) {
                if term < terms.len() {
                    terms[term].constraint = MatchTermConstraint::Constant(value);
                    return changed_node(node);
                }
            };
            return None;
        }
        Mutation::SetConstraintFree {
            term: Term(node, term),
        } => {
            if let &mut Node::Match { ref mut terms, .. } = diagram.get_node_mut(node) {
                if term < terms.len() {
                    terms[term].constraint = MatchTermConstraint::Free;
                    return changed_node(node);
                }
            };
            return None;
        }
        Mutation::SetTarget {
            term: Term(node, term),
            register,
        } => {
            if let &mut Node::Match { ref mut terms, .. } = diagram.get_node_mut(node) {
                if term < terms.len() {
                    terms[term].target = register;
                    return changed_node(node);
                }
            };
            return None;
        }
        Mutation::InsertPassthrough {
            predicate,
            num_terms,
            edge,
        } => {
            let node = Node::Match {
                predicate,
                terms: iter::repeat(MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: None,
                }).take(num_terms)
                    .collect(),
            };
            let node_index = diagram.insert_node(node);
            match edge {
                Edge::Root => {
                    let target = diagram.get_root();
                    diagram.set_on_match(node_index, target);
                    diagram.set_on_refute(node_index, target);
                    diagram.set_root(node_index);
                }
                Edge::Match(src) => {
                    if let Some(target) = diagram.get_on_match(src) {
                        diagram.set_on_match(node_index, target);
                        diagram.set_on_refute(node_index, target);
                    }
                    diagram.set_on_match(src, node_index);
                }
                Edge::Refute(src) => {
                    if let Some(target) = diagram.get_on_refute(src) {
                        diagram.set_on_match(node_index, target);
                        diagram.set_on_refute(node_index, target);
                    }
                    diagram.set_on_refute(src, node_index);
                }
            }
            return Some(MutationResult {
                phenotype_could_have_changed: false,
                node_to_restart: None,
            });
        }
        Mutation::RemoveNode { node } => {
            let node_could_be_passthrough =
                if let Node::Match { ref terms, .. } = *diagram.get_node(node) {
                    terms.iter().all(|term| term.target.is_none())
                } else {
                    false
                };
            let mut had_sources = false;
            let maybe_match = diagram.get_on_match(node);
            let maybe_refute = diagram.get_on_refute(node);
            if let Some(on_match) = maybe_match {
                if let Some(match_sources) =
                    diagram.get_match_sources(node).map(|srcs| srcs.to_owned())
                {
                    for src in match_sources {
                        had_sources = true;
                        diagram.set_on_match(src, on_match);
                    }
                }
            } else {
                if let Some(match_sources) =
                    diagram.get_match_sources(node).map(|srcs| srcs.to_owned())
                {
                    for src in match_sources {
                        had_sources = true;
                        diagram.clear_on_match(src);
                    }
                }
            };
            if let Some(on_refute) = maybe_refute {
                if let Some(refute_sources) =
                    diagram.get_refute_sources(node).map(|srcs| srcs.to_owned())
                {
                    for src in refute_sources {
                        had_sources = true;
                        diagram.set_on_refute(src, on_refute);
                    }
                }
            } else {
                if let Some(refute_sources) =
                    diagram.get_refute_sources(node).map(|srcs| srcs.to_owned())
                {
                    for src in refute_sources {
                        had_sources = true;
                        diagram.clear_on_refute(src);
                    }
                }
            };
            if maybe_match == maybe_refute && node_could_be_passthrough {
                return Some(MutationResult {
                    phenotype_could_have_changed: false,
                    node_to_restart: None,
                });
            } else {
                // TODO(zentner): Check for parallel sibling?
                return Some(MutationResult {
                    phenotype_could_have_changed: had_sources,
                    node_to_restart: None,
                });
            }
        }
        Mutation::DuplicateTarget { node } => {
            let maybe_match = diagram.get_on_match(node);
            let maybe_refute = diagram.get_on_refute(node);
            match (maybe_match, maybe_refute) {
                (Some(on_match), Some(on_refute)) if on_match == on_refute => {
                    let target = on_match;
                    let duplicate = diagram.get_node(target).clone();
                    let duplicate = diagram.insert_node(duplicate);
                    diagram.set_on_match(node, duplicate);
                    if let Some(target_on_match) = diagram.get_on_match(target) {
                        diagram.set_on_match(duplicate, target_on_match);
                    }
                    if let Some(target_on_refute) = diagram.get_on_refute(target) {
                        diagram.set_on_refute(duplicate, target_on_refute);
                    }
                    return Some(MutationResult {
                        phenotype_could_have_changed: false,
                        node_to_restart: None,
                    });
                }
                _ => {
                    return None;
                }
            }
        }
        Mutation::SetEdge { edge, target } => match edge {
            Edge::Root => {
                diagram.set_root(target);
                Some(MutationResult {
                    phenotype_could_have_changed: true,
                    node_to_restart: None,
                })
            }
            Edge::Match(src) => {
                diagram.set_on_match(src, target);
                Some(MutationResult {
                    phenotype_could_have_changed: true,
                    node_to_restart: Some(src),
                })
            }
            Edge::Refute(src) => {
                diagram.set_on_match(src, target);
                Some(MutationResult {
                    phenotype_could_have_changed: true,
                    node_to_restart: Some(src),
                })
            }
        },
        Mutation::SetOutputRegister {
            term: Term(node, term),
            register,
        } => {
            if let Node::Output { ref mut terms, .. } = *diagram.get_node_mut(node) {
                terms[term] = OutputTerm::Register(register);
                Some(MutationResult {
                    phenotype_could_have_changed: true,
                    node_to_restart: Some(node),
                })
            } else {
                None
            }
        }
        Mutation::SetOutputConstant {
            term: Term(node, term),
            value,
        } => {
            if let Node::Output { ref mut terms, .. } = *diagram.get_node_mut(node) {
                terms[term] = OutputTerm::Constant(value);
                Some(MutationResult {
                    phenotype_could_have_changed: true,
                    node_to_restart: Some(node),
                })
            } else {
                None
            }
        }
        Mutation::SetPredicate { node, predicate } => match *diagram.get_node_mut(node) {
            Node::Output {
                predicate: ref mut p,
                ..
            }
            | Node::Match {
                predicate: ref mut p,
                ..
            } => {
                *p = predicate;
                Some(MutationResult {
                    phenotype_could_have_changed: true,
                    node_to_restart: Some(node),
                })
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram::{MatchTerm, MatchTermConstraint, OutputTerm};
    use graph_diagram::GraphDiagram;
    use predicate::Predicate;
    use value::Value;

    #[test]
    fn can_set_constraint_register() {
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
        apply_mutation(
            &mut diagram,
            Mutation::SetConstraintRegister {
                term: Term(root, 0),
                register: 0,
            },
        );
        assert_eq!(
            *diagram.get_node(root),
            Node::Match {
                predicate: Predicate(0),
                terms: vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Register(0),
                        target: Some(0),
                    },
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: Some(1),
                    },
                ],
            }
        );
    }

    #[test]
    fn can_set_constraint_constant() {
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
        apply_mutation(
            &mut diagram,
            Mutation::SetConstraintConstant {
                term: Term(root, 0),
                value: Value::Symbol(0),
            },
        );
        assert_eq!(
            *diagram.get_node(root),
            Node::Match {
                predicate: Predicate(0),
                terms: vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Constant(Value::Symbol(0)),
                        target: Some(0),
                    },
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: Some(1),
                    },
                ],
            }
        );
    }

    #[test]
    fn can_set_constraint_free() {
        let mut diagram = GraphDiagram::new(2);
        let match_anything_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm {
                    constraint: MatchTermConstraint::Constant(Value::Symbol(0)),
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
        apply_mutation(
            &mut diagram,
            Mutation::SetConstraintFree {
                term: Term(root, 0),
            },
        );
        assert_eq!(
            *diagram.get_node(root),
            Node::Match {
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
            }
        );
    }
}
