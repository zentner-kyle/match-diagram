use rand::Rng;
use std::collections::HashMap;

use diagram::{DiagramSpace, Edge, EdgeGroup, MultiDiagram, OutputTerm};
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

    fn gen_node<R: Rng>(&self, rng: &mut R, state: &mut IndividualMutationState) -> NodeIndex {
        loop {
            let node = NodeIndex(rng.gen_range(0, self.diagram.len()));
            if state
                .deleted_nodes
                .iter()
                .position(|n| *n == node)
                .is_none()
            {
                return node;
            }
        }
    }

    fn gen_term<R: Rng>(&self, rng: &mut R, state: &mut IndividualMutationState) -> Term {
        let register = rng.gen_range(0, self.space.num_terms);
        Term(self.gen_node(rng, state), register)
    }

    fn gen_value<R: Rng>(&self, rng: &mut R) -> Value {
        choose_from_iter(rng, self.frame.values.iter())
            .expect("space should have at least on value")
            .clone()
    }

    fn gen_register<R: Rng>(&self, rng: &mut R) -> usize {
        rng.gen_range(0, self.space.num_registers)
    }

    fn gen_edge<R: Rng>(&self, rng: &mut R, state: &mut IndividualMutationState) -> Edge {
        match rng.gen_range(0, 3) {
            0 => Edge::Root(self.gen_node(rng, state)),
            1 => Edge::Match {
                source: self.gen_node(rng, state),
                target: self.gen_node(rng, state),
            },
            2 => Edge::Refute {
                source: self.gen_node(rng, state),
                target: self.gen_node(rng, state),
            },
            _ => unreachable!(),
        }
    }

    fn gen_group<R: Rng>(&self, rng: &mut R, state: &mut IndividualMutationState) -> EdgeGroup {
        match rng.gen_range(0, 3) {
            0 => EdgeGroup::Roots,
            1 => EdgeGroup::MatchTargets(self.gen_node(rng, state)),
            2 => EdgeGroup::RefuteTargets(self.gen_node(rng, state)),
            _ => unreachable!(),
        }
    }

    fn gen_predicate<R: Rng>(&self, rng: &mut R) -> Predicate {
        Predicate(rng.gen_range(0, self.frame.num_terms_for_predicate.len() as u64))
    }

    fn gen_output_terms<R: Rng>(&self, rng: &mut R, predicate: Predicate) -> Vec<OutputTerm> {
        let num_terms = *self.frame
            .num_terms_for_predicate
            .get(&predicate)
            .expect("should have only generated a known predicate");
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
}

impl<'f, 's, 'd, D: 'd + MultiDiagram> GenMutation for UniformMutationContext<'f, 's, 'd, D> {
    fn gen_mutation<R: Rng>(&self, state: &mut IndividualMutationState, rng: &mut R) -> Mutation {
        match rng.gen_range(0, 10) {
            0 => Mutation::SetConstraintRegister {
                term: self.gen_term(rng, state),
                register: self.gen_register(rng),
            },
            1 => Mutation::SetConstraintConstant {
                term: self.gen_term(rng, state),
                value: self.gen_value(rng),
            },
            2 => Mutation::SetConstraintFree {
                term: self.gen_term(rng, state),
            },
            3 => Mutation::SetTarget {
                term: self.gen_term(rng, state),
                register: if rng.gen() {
                    Some(self.gen_register(rng))
                } else {
                    None
                },
            },
            4 => Mutation::InsertEdge {
                edge: self.gen_edge(rng, state),
            },
            5 => Mutation::SetOutputRegister {
                term: self.gen_term(rng, state),
                register: self.gen_register(rng),
            },
            6 => Mutation::SetOutputConstant {
                term: self.gen_term(rng, state),
                value: self.gen_value(rng),
            },
            7 => Mutation::SetPredicate {
                node: self.gen_node(rng, state),
                predicate: self.gen_predicate(rng),
            },
            8 => Mutation::RemoveNode {
                node: self.gen_node(rng, state),
            },
            9 => {
                let predicate = self.gen_predicate(rng);
                Mutation::InsertOutputNode {
                    group: self.gen_group(rng, state),
                    predicate,
                    terms: self.gen_output_terms(rng, predicate),
                }
            }
            _ => unreachable!(),
        }
    }
}
