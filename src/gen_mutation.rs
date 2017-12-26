use rand::Rng;

use fixgraph::NodeIndex;
use mutation::{Edge, Mutation, Term};
use predicate::Predicate;
use value::Value;

pub struct UniformMutationContext {
    num_nodes: usize,
    num_terms: usize,
    num_registers: usize,
    num_symbols: u64,
    num_predicates: u64,
}

pub trait GenMutation {
    fn gen_mutation<R: Rng>(&mut self, rng: &mut R) -> Mutation;
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
    ) -> Self {
        UniformMutationContext {
            num_nodes: nonzero(num_nodes),
            num_terms: nonzero(num_terms),
            num_registers: nonzero(num_registers),
            num_symbols: nonzero_u64(num_symbols),
            num_predicates: nonzero_u64(num_predicates),
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
            0 => Edge::Root,
            1 => Edge::Match(self.gen_node(rng)),
            2 => Edge::Refute(self.gen_node(rng)),
            _ => unreachable!(),
        }
    }

    fn gen_predicate<R: Rng>(&self, rng: &mut R) -> Predicate {
        Predicate(rng.gen_range(0, self.num_predicates))
    }
}

impl GenMutation for UniformMutationContext {
    fn gen_mutation<R: Rng>(&mut self, rng: &mut R) -> Mutation {
        match rng.gen_range(0, 11) {
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
            4 => Mutation::InsertPassthrough {
                predicate: self.gen_predicate(rng),
                edge: self.gen_edge(rng),
            },
            5 => Mutation::RemoveNode {
                node: self.gen_node(rng),
            },
            6 => Mutation::DuplicateTarget {
                node: self.gen_node(rng),
            },
            7 => Mutation::SetEdge {
                edge: self.gen_edge(rng),
                target: self.gen_node(rng),
            },
            8 => Mutation::SetOutputRegister {
                term: self.gen_term(rng),
                register: self.gen_register(rng),
            },
            9 => Mutation::SetOutputConstant {
                term: self.gen_term(rng),
                value: self.gen_value(rng),
            },
            10 => Mutation::SetPredicate {
                node: self.gen_node(rng),
                predicate: self.gen_predicate(rng),
            },
            _ => unreachable!(),
        }
    }
}
