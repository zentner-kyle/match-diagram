use rand::Rng;

use diagram::{Edge, EdgeGroup, MultiDiagram};
use mutation::{Mutation, Term};
use node_index::NodeIndex;
use predicate::Predicate;
use std::collections::HashMap;
use value::Value;

#[derive(Debug, Clone)]
pub struct IndividualMutationState {}

impl IndividualMutationState {
    pub fn new() -> Self {
        IndividualMutationState {}
    }
}

#[derive(Debug, Clone)]
pub struct UniformMutationContext {
    num_nodes: usize,
    num_terms: usize,
    num_registers: usize,
    num_symbols: u64,
    num_predicates: u64,
    num_terms_for_predicate: HashMap<Predicate, usize>,
}

pub trait GenMutation {
    fn gen_mutation<D: MultiDiagram, R: Rng>(
        &self,
        diagram: &D,
        state: &mut IndividualMutationState,
        rng: &mut R,
    ) -> Mutation;
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

impl UniformMutationContext {
    pub fn new(
        num_nodes: usize,
        num_terms: usize,
        num_registers: usize,
        num_symbols: u64,
        num_predicates: u64,
        num_terms_for_predicate: HashMap<Predicate, usize>,
    ) -> Self {
        UniformMutationContext {
            num_nodes: nonzero(num_nodes),
            num_terms: nonzero(num_terms),
            num_registers: nonzero(num_registers),
            num_symbols: nonzero_u64(num_symbols),
            num_predicates: nonzero_u64(num_predicates),
            num_terms_for_predicate,
        }
    }

    fn gen_node<R: Rng>(&self, rng: &mut R) -> NodeIndex {
        NodeIndex(rng.gen_range(0, self.num_nodes))
    }

    fn gen_term<R: Rng>(&self, rng: &mut R) -> Term {
        let register = rng.gen_range(0, self.num_terms);
        Term(self.gen_node(rng), register)
    }

    fn gen_value<R: Rng>(&self, rng: &mut R) -> Value {
        Value::Symbol(rng.gen_range(0, self.num_symbols))
    }

    fn gen_register<R: Rng>(&self, rng: &mut R) -> usize {
        rng.gen_range(0, self.num_registers)
    }

    fn gen_edge<R: Rng>(&self, rng: &mut R) -> Edge {
        match rng.gen_range(0, 3) {
            0 => Edge::Root(self.gen_node(rng)),
            1 => Edge::Match {
                source: self.gen_node(rng),
                target: self.gen_node(rng),
            },
            2 => Edge::Refute {
                source: self.gen_node(rng),
                target: self.gen_node(rng),
            },
            _ => unreachable!(),
        }
    }

    fn gen_group<R: Rng>(&self, rng: &mut R) -> EdgeGroup {
        match rng.gen_range(0, 3) {
            0 => EdgeGroup::Roots,
            1 => EdgeGroup::MatchTargets(self.gen_node(rng)),
            2 => EdgeGroup::RefuteTargets(self.gen_node(rng)),
            _ => unreachable!(),
        }
    }

    fn gen_predicate<R: Rng>(&self, rng: &mut R) -> Predicate {
        Predicate(rng.gen_range(0, self.num_predicates))
    }
}

impl GenMutation for UniformMutationContext {
    fn gen_mutation<D: MultiDiagram, R: Rng>(
        &self,
        diagram: &D,
        state: &mut IndividualMutationState,
        rng: &mut R,
    ) -> Mutation {
        match rng.gen_range(0, 10) {
            0 => Mutation::SetConstraintRegister {
                term: self.gen_term(rng),
                register: self.gen_register(rng),
            },
            1 => Mutation::SetConstraintConstant {
                term: self.gen_term(rng),
                value: self.gen_value(rng),
            },
            2 => Mutation::SetConstraintFree {
                term: self.gen_term(rng),
            },
            3 => Mutation::SetTarget {
                term: self.gen_term(rng),
                register: if rng.gen() {
                    Some(self.gen_register(rng))
                } else {
                    None
                },
            },
            4 => Mutation::InsertEdge {
                edge: self.gen_edge(rng),
            },
            5 => Mutation::SetOutputRegister {
                term: self.gen_term(rng),
                register: self.gen_register(rng),
            },
            6 => Mutation::SetOutputConstant {
                term: self.gen_term(rng),
                value: self.gen_value(rng),
            },
            7 => Mutation::SetPredicate {
                node: self.gen_node(rng),
                predicate: self.gen_predicate(rng),
            },
            8 => Mutation::RemoveNode {
                node: self.gen_node(rng),
            },
            _ => unreachable!(),
        }
    }
}
