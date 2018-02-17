use rand::Rng;
use std::collections::HashMap;

use diagram::{DiagramSpace, Edge, EdgeGroup, MatchTerm, MatchTermConstraint, MultiDiagram, Node,
              OutputTerm};
use frame::Frame;
use mutation::{Mutation, Term};
use node_index::NodeIndex;
use predicate::Predicate;
use rand_utils::choose_from_iter;
use value::Value;

#[derive(Debug, Clone)]
pub struct IndividualMutationState {
    pub deleted_nodes: Vec<NodeIndex>,
}

impl IndividualMutationState {
    pub fn new() -> Self {
        IndividualMutationState {
            deleted_nodes: Vec::new(),
        }
    }

    pub fn insert_node<D: MultiDiagram>(&mut self, diagram: &mut D, node: Node) -> NodeIndex {
        if let Some(deleted) = self.deleted_nodes.pop() {
            *diagram.get_node_mut(deleted) = node;
            deleted
        } else {
            diagram.insert_node(node)
        }
    }
}

#[derive(Debug, Clone)]
pub struct UniformMutationContext<'f, 's, 'd, D: 'd + MultiDiagram> {
    frame: &'f Frame,
    space: &'s DiagramSpace,
    diagram: &'d D,
}

pub trait GenMutation {
    fn gen_mutation<R: Rng>(&self, state: &mut IndividualMutationState, rng: &mut R) -> Mutation;
}

fn nonzero(value: usize) -> usize {
    if value == 0 {
        1
    } else {
        value
    }
}

fn nonzero_u64(value: u64) -> u64 {
    if value == 0 {
        1
    } else {
        value
    }
}

impl<'f, 's, 'd, D: 'd + MultiDiagram> UniformMutationContext<'f, 's, 'd, D> {
    pub fn new(frame: &'f Frame, space: &'s DiagramSpace, diagram: &'d D) -> Self {
        UniformMutationContext {
            frame,
            space,
            diagram,
        }
    }

    fn gen_node<R: Rng>(
        &self,
        rng: &mut R,
        state: &mut IndividualMutationState,
    ) -> Option<NodeIndex> {
        if self.diagram.len() <= state.deleted_nodes.len() {
            return None;
        }
        loop {
            let node = NodeIndex(rng.gen_range(0, self.diagram.len()));
            if state
                .deleted_nodes
                .iter()
                .position(|n| *n == node)
                .is_none()
            {
                return Some(node);
            }
        }
    }

    fn gen_term<R: Rng>(&self, rng: &mut R, state: &mut IndividualMutationState) -> Option<Term> {
        let register = rng.gen_range(0, self.space.num_terms);
        Some(Term(self.gen_node(rng, state)?, register))
    }

    fn gen_value<R: Rng>(&self, rng: &mut R) -> Value {
        choose_from_iter(rng, self.frame.values.iter())
            .expect("space should have at least on value")
            .clone()
    }

    fn gen_register<R: Rng>(&self, rng: &mut R) -> usize {
        assert!(self.space.num_registers != 0, "need at least one register");
        rng.gen_range(0, self.space.num_registers)
    }

    fn gen_edge<R: Rng>(&self, rng: &mut R, state: &mut IndividualMutationState) -> Option<Edge> {
        match rng.gen_range(0, 3) {
            0 => Some(Edge::Root(self.gen_node(rng, state)?)),
            1 => Some(Edge::Match {
                source: self.gen_node(rng, state)?,
                target: self.gen_node(rng, state)?,
            }),
            2 => Some(Edge::Refute {
                source: self.gen_node(rng, state)?,
                target: self.gen_node(rng, state)?,
            }),
            _ => unreachable!(),
        }
    }

    fn gen_group<R: Rng>(
        &self,
        rng: &mut R,
        state: &mut IndividualMutationState,
    ) -> Option<EdgeGroup> {
        match rng.gen_range(0, 3) {
            0 => Some(EdgeGroup::Roots),
            1 => Some(EdgeGroup::MatchTargets(self.gen_node(rng, state)?)),
            2 => Some(EdgeGroup::RefuteTargets(self.gen_node(rng, state)?)),
            _ => unreachable!(),
        }
    }

    fn gen_predicate<R: Rng>(&self, rng: &mut R) -> Predicate {
        Predicate(rng.gen_range(0, self.frame.num_terms_for_predicate.len() as u64))
    }

    fn get_num_terms(&self, predicate: Predicate) -> usize {
        let num_terms = *self.frame
            .num_terms_for_predicate
            .get(&predicate)
            .expect("should have only generated a known predicate");
        assert!(
            num_terms != 0,
            "all predicates should have at least one term"
        );
        num_terms
    }

    fn gen_output_terms<R: Rng>(&self, rng: &mut R, predicate: Predicate) -> Vec<OutputTerm> {
        let num_terms = self.get_num_terms(predicate);
        let mut output = Vec::with_capacity(num_terms);
        for _ in 0..num_terms {
            if rng.gen() {
                let register = self.gen_register(rng);
                output.push(OutputTerm::Register(register));
            } else {
                let value = self.gen_value(rng);
                output.push(OutputTerm::Constant(value));
            }
        }
        output
    }

    fn gen_match_terms<R: Rng>(&self, rng: &mut R, predicate: Predicate) -> Vec<MatchTerm> {
        let num_terms = self.get_num_terms(predicate);
        let mut output = Vec::with_capacity(num_terms);
        for _ in 0..num_terms {
            match rng.gen_range(0, 3) {
                0 => {
                    let register = self.gen_register(rng);
                    output.push(MatchTerm {
                        constraint: MatchTermConstraint::Register(register),
                        target: None,
                    });
                }
                1 => {
                    let value = self.gen_value(rng);
                    output.push(MatchTerm {
                        constraint: MatchTermConstraint::Constant(value),
                        target: None,
                    });
                }
                2 => {
                    output.push(MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: None,
                    });
                }
                _ => unreachable!(),
            }
        }
        output
    }

    fn pick_edge<R: Rng>(&self, rng: &mut R, state: &mut IndividualMutationState) -> Option<Edge> {
        let target = self.gen_node(rng, state)?;
        match rng.gen_range(0, 3) {
            0 => {
                if self.diagram.edge_exists(Edge::Root(target)) {
                    return Some(Edge::Root(target));
                }
            }
            1 => {
                let group = self.diagram.get_group(EdgeGroup::MatchSources(target));
                if group.len() > 0 {
                    let source = group[rng.gen_range(0, group.len())];
                    return Some(Edge::Match { source, target });
                }
            }
            2 => {
                let group = self.diagram.get_group(EdgeGroup::RefuteSources(target));
                if group.len() > 0 {
                    let source = group[rng.gen_range(0, group.len())];
                    return Some(Edge::Refute { source, target });
                }
            }
            _ => unreachable!(),
        }
        return None;
    }

    fn gen_mutation_inner<R: Rng>(
        &self,
        state: &mut IndividualMutationState,
        rng: &mut R,
    ) -> Option<Mutation> {
        match rng.gen_range(0, 11) {
            0 => Some(Mutation::SetConstraintRegister {
                term: self.gen_term(rng, state)?,
                register: self.gen_register(rng),
            }),
            1 => Some(Mutation::SetConstraintConstant {
                term: self.gen_term(rng, state)?,
                value: self.gen_value(rng),
            }),
            2 => Some(Mutation::SetConstraintFree {
                term: self.gen_term(rng, state)?,
            }),
            3 => Some(Mutation::SetTarget {
                term: self.gen_term(rng, state)?,
                register: if rng.gen() {
                    Some(self.gen_register(rng))
                } else {
                    None
                },
            }),
            4 => Some(Mutation::InsertEdge {
                edge: self.gen_edge(rng, state)?,
            }),
            5 => Some(Mutation::SetOutputRegister {
                term: self.gen_term(rng, state)?,
                register: self.gen_register(rng),
            }),
            6 => Some(Mutation::SetOutputConstant {
                term: self.gen_term(rng, state)?,
                value: self.gen_value(rng),
            }),
            7 => Some(Mutation::SetPredicate {
                node: self.gen_node(rng, state)?,
                predicate: self.gen_predicate(rng),
            }),
            8 => Some(Mutation::RemoveNode {
                node: self.gen_node(rng, state)?,
            }),
            9 => {
                let predicate = self.gen_predicate(rng);
                Some(Mutation::InsertOutputNode {
                    group: self.gen_group(rng, state)?,
                    predicate,
                    terms: self.gen_output_terms(rng, predicate),
                })
            }
            10 => {
                let predicate = self.gen_predicate(rng);
                Some(Mutation::InsertMatchNode {
                    edge: self.pick_edge(rng, state)?,
                    predicate,
                    terms: self.gen_match_terms(rng, predicate),
                })
            }
            _ => unreachable!(),
        }
    }
}

impl<'f, 's, 'd, D: 'd + MultiDiagram> GenMutation for UniformMutationContext<'f, 's, 'd, D> {
    fn gen_mutation<R: Rng>(&self, state: &mut IndividualMutationState, rng: &mut R) -> Mutation {
        loop {
            if let Some(mutation) = self.gen_mutation_inner(state, rng) {
                return mutation;
            }
        }
    }
}
