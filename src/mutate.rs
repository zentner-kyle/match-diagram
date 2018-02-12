use diagram::{Diagram, Edge, EdgeGroup, MatchTerm, MatchTermConstraint, MultiDiagram, Node,
              OutputTerm};
use mutation::{Mutation, Term};
use node_index::NodeIndex;
use std::iter;

#[derive(Debug, PartialEq, Eq)]
pub struct MutationResult {
    pub phenotype_could_have_changed: bool,
    pub node_to_restart: Option<NodeIndex>,
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
        Mutation::RemoveNode { node } => {
            let was_root = diagram
                .get_group(EdgeGroup::Roots)
                .iter()
                .position(|n| *n == node)
                .is_some();
            let match_sources = diagram.get_group(EdgeGroup::MatchSources(node)).to_owned();
            let match_targets = diagram.get_group(EdgeGroup::MatchTargets(node)).to_owned();
            let refute_sources = diagram.get_group(EdgeGroup::RefuteSources(node)).to_owned();
            let refute_targets = diagram.get_group(EdgeGroup::RefuteTargets(node)).to_owned();

            for target in match_targets
                .iter()
                .cloned()
                .chain(refute_targets.iter().cloned())
            {
                for source in match_sources.iter().cloned() {
                    diagram.insert_edge(Edge::Match { source, target });
                }
                for source in refute_sources.iter().cloned() {
                    diagram.insert_edge(Edge::Refute { source, target });
                }
            }

            if was_root {
                for target in match_targets
                    .iter()
                    .cloned()
                    .chain(refute_targets.iter().cloned())
                {
                    diagram.insert_edge(Edge::Root(target));
                }
            }

            for source in match_sources.iter().cloned() {
                diagram.remove_edge(Edge::Match {
                    source,
                    target: node,
                });
            }
            for target in match_targets.iter().cloned() {
                diagram.remove_edge(Edge::Match {
                    source: node,
                    target,
                });
            }
            for source in refute_sources.iter().cloned() {
                diagram.remove_edge(Edge::Refute {
                    source,
                    target: node,
                });
            }
            for target in refute_targets.iter().cloned() {
                diagram.remove_edge(Edge::Refute {
                    source: node,
                    target,
                });
            }

            let had_sources = was_root || match_sources.len() != 0 || refute_sources.len() != 0;

            return Some(MutationResult {
                phenotype_could_have_changed: had_sources,
                node_to_restart: None,
            });
        }
        Mutation::InsertEdge { edge } => {
            diagram.insert_edge(edge);
            return Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: edge.source(),
            });
        }
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
        Mutation::SetPredicate { node, predicate } => {
            return match *diagram.get_node_mut(node) {
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
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram::{EdgeGroup, MatchTerm, MatchTermConstraint, OutputTerm};
    use graph_diagram::GraphDiagram;
    use parse::{node_literal, parse_diagram};
    use predicate::Predicate;
    use value::Value;

    fn diagram_literal(src: &str, num_registers: usize) -> GraphDiagram {
        parse_diagram(src, num_registers).unwrap().0
    }

    #[test]
    fn can_set_constraint_register() {
        let mut diagram = diagram_literal(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          output @1(%0, %1)
        }
        "#,
            2,
        );
        let root = diagram.get_root();
        apply_mutation(
            &mut diagram,
            Mutation::SetConstraintRegister {
                term: Term(root, 0),
                register: 0,
            },
        );
        assert_eq!(
            *diagram.get_node(root),
            node_literal("@0(%0 -> %0, _ -> %1)")
        );
    }

    #[test]
    fn can_set_constraint_constant() {
        let mut diagram = diagram_literal(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          output @1(%0, %1)
        }
        "#,
            2,
        );
        let root = diagram.get_root();
        apply_mutation(
            &mut diagram,
            Mutation::SetConstraintConstant {
                term: Term(root, 0),
                value: Value::Symbol(0),
            },
        );
        assert_eq!(
            *diagram.get_node(root),
            node_literal("@0(:0 -> %0, _ -> %1)")
        );
    }

    #[test]
    fn can_set_constraint_free() {
        let mut diagram = diagram_literal(
            r#"
        root: @0(:0 -> %0, _ -> %1) {
          output @1(%0, %1)
        }
        "#,
            2,
        );
        let root = diagram.get_root();
        apply_mutation(
            &mut diagram,
            Mutation::SetConstraintFree {
                term: Term(root, 0),
            },
        );
        assert_eq!(
            *diagram.get_node(root),
            node_literal("@0(_ -> %0, _ -> %1)")
        );
    }

    #[test]
    fn set_target() {
        let mut diagram = diagram_literal(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          output @1(%0, %1)
        }
        "#,
            2,
        );
        let root = diagram.get_root();
        apply_mutation(
            &mut diagram,
            Mutation::SetTarget {
                term: Term(root, 0),
                register: None,
            },
        );
        assert_eq!(*diagram.get_node(root), node_literal("@0(_, _ -> %1)"));
    }

    #[test]
    fn remove_node_not_passthrough() {
        let (mut diagram, context) = parse_diagram(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          a: @1(_ -> %0, _ -> %1) {
            b: output @2(%0, %1)
          } { b }
        } { a }
        "#,
            2,
        ).unwrap();
        println!("original diagram = {:#?}", diagram);
        let root = diagram.get_root();
        let a = context.node_name_to_info.get("a").unwrap().index;
        let mutation_result = apply_mutation(&mut diagram, Mutation::RemoveNode { node: a });
        println!("mutated diagram = {:#?}", diagram);
        assert_eq!(
            mutation_result,
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: None,
            })
        );
        let b = context.node_name_to_info.get("b").unwrap().index;
        println!("root = {:?}", root);
        assert_eq!(diagram.get_on_match(root), Some(b));
        assert_eq!(diagram.get_on_match(root), Some(b));
    }

    #[test]
    fn remove_node_root() {
        let mut diagram = diagram_literal(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          a: output @1(%0, %1)
        }
        "#,
            2,
        );
        let root = diagram.get_root();
        assert_eq!(
            apply_mutation(&mut diagram, Mutation::RemoveNode { node: root }),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: None,
            })
        );
    }

    #[test]
    fn set_edge_root() {
        let (mut diagram, context) = parse_diagram(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          a: @1(_, _) {
            b: output @2(%0, %1)
          } { b }
        } { a }
        "#,
            2,
        ).unwrap();
        let a = context.node_name_to_info.get("a").unwrap().index;
        assert_eq!(
            apply_mutation(
                &mut diagram,
                Mutation::InsertEdge {
                    edge: Edge::Root(a),
                }
            ),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: None,
            })
        );
        assert!(
            diagram
                .get_group(EdgeGroup::Roots)
                .iter()
                .position(|n| *n == a)
                .is_some()
        );
    }

    #[test]
    fn set_edge_match() {
        let (mut diagram, context) = parse_diagram(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          a: @1(_, _) {
            b: output @2(%0, %1)
          } { b }
        } { a }
        "#,
            2,
        ).unwrap();
        let a = context.node_name_to_info.get("a").unwrap().index;
        let b = context.node_name_to_info.get("b").unwrap().index;
        assert_eq!(
            apply_mutation(
                &mut diagram,
                Mutation::InsertEdge {
                    edge: Edge::Match {
                        source: a,
                        target: a,
                    },
                }
            ),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: Some(a),
            })
        );
        assert!(
            diagram
                .get_group(EdgeGroup::MatchTargets(a))
                .iter()
                .position(|n| *n == a)
                .is_some()
        );
        assert!(
            diagram
                .get_group(EdgeGroup::RefuteTargets(a))
                .iter()
                .position(|n| *n == b)
                .is_some()
        );
    }

    #[test]
    fn set_edge_refute() {
        let (mut diagram, context) = parse_diagram(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          a: @1(_, _) {
            b: output @2(%0, %1)
          } { b }
        } { a }
        "#,
            2,
        ).unwrap();
        let a = context.node_name_to_info.get("a").unwrap().index;
        let b = context.node_name_to_info.get("b").unwrap().index;
        assert_eq!(
            apply_mutation(
                &mut diagram,
                Mutation::InsertEdge {
                    edge: Edge::Refute {
                        source: a,
                        target: a,
                    },
                }
            ),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: Some(a),
            })
        );
        assert!(
            diagram
                .get_group(EdgeGroup::RefuteTargets(a))
                .iter()
                .position(|n| *n == a)
                .is_some()
        );
        assert!(
            diagram
                .get_group(EdgeGroup::MatchTargets(a))
                .iter()
                .position(|n| *n == b)
                .is_some()
        );
    }

    #[test]
    fn set_output_register() {
        let mut diagram = diagram_literal(
            r#"
        root: output @1(:2, :2) 
        "#,
            2,
        );
        let root = diagram.get_root();
        assert_eq!(
            apply_mutation(
                &mut diagram,
                Mutation::SetOutputRegister {
                    term: Term(root, 0),
                    register: 1,
                }
            ),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: Some(root),
            })
        );
        assert_eq!(*diagram.get_node(root), node_literal("output @1(%1, :2)"));
    }

    #[test]
    fn set_output_constant() {
        let mut diagram = diagram_literal(
            r#"
        root: output @1(:2, :2) 
        "#,
            2,
        );
        let root = diagram.get_root();
        assert_eq!(
            apply_mutation(
                &mut diagram,
                Mutation::SetOutputConstant {
                    term: Term(root, 0),
                    value: Value::Symbol(1),
                }
            ),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: Some(root),
            })
        );
        assert_eq!(*diagram.get_node(root), node_literal("output @1(:1, :2)"));
    }

    #[test]
    fn set_predicate_output() {
        let mut diagram = diagram_literal(
            r#"
        root: output @1(:2, :2) 
        "#,
            2,
        );
        let root = diagram.get_root();
        assert_eq!(
            apply_mutation(
                &mut diagram,
                Mutation::SetPredicate {
                    node: root,
                    predicate: Predicate(0),
                }
            ),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: Some(root),
            })
        );
        assert_eq!(*diagram.get_node(root), node_literal("output @0(:2, :2)"));
    }

    #[test]
    fn set_predicate_match() {
        let mut diagram = diagram_literal(
            r#"
        root: @0(_ -> %0, _ -> %1) {
          a: output @1(%0, %1)
        }
        "#,
            2,
        );
        let root = diagram.get_root();
        assert_eq!(
            apply_mutation(
                &mut diagram,
                Mutation::SetPredicate {
                    node: root,
                    predicate: Predicate(1),
                }
            ),
            Some(MutationResult {
                phenotype_could_have_changed: true,
                node_to_restart: Some(root),
            })
        );
        assert_eq!(
            *diagram.get_node(root),
            node_literal("@1(_ -> %0, _ -> %1)")
        );
    }
}
