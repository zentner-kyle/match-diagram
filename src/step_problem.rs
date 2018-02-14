use evolution_strategies::Problem;
use rand::Rng;
use std::cmp::{Ordering, PartialOrd};
use std::iter;

use database::Database;
use diagram::{Diagram, DiagramSpace, MultiDiagram, Node, OutputTerm};
use evaluation::Evaluation;
use frame::Frame;
use gen_mutation::{GenMutation, IndividualMutationState, UniformMutationContext};
use graph_diagram::GraphDiagram;
use mutate::{apply_mutation, MutationResult};
use node_index::NodeIndex;
use predicate::Predicate;
use value::Value;

#[derive(Clone, Debug)]
pub struct DiagramIndividual {
    pub diagram: GraphDiagram,
    pub evaluations: Vec<Evaluation>,
    pub fitness: i64,
    pub mutation_state: IndividualMutationState,
}

impl DiagramIndividual {
    fn blank(
        num_evaluations: usize,
        num_registers: usize,
        num_nodes: usize,
        num_0_terms: usize,
    ) -> DiagramIndividual {
        let mut diagram = GraphDiagram::new(num_registers);

        for _ in 0..num_nodes {
            diagram.insert_node(Node::Output {
                predicate: Predicate(0),
                terms: iter::repeat(OutputTerm::Constant(Value::Symbol(0)))
                    .take(num_0_terms)
                    .collect(),
            });
        }
        diagram.set_root(NodeIndex(0));
        let evaluations = iter::repeat(Evaluation::new())
            .take(num_evaluations)
            .collect();
        DiagramIndividual {
            diagram,
            evaluations,
            fitness: i64::min_value(),
            mutation_state: IndividualMutationState::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StepProblem {
    samples: Vec<(Database, Database)>,
    frame: Frame,
    space: DiagramSpace,
    num_registers: usize,
    num_nodes: usize,
    num_0_terms: usize,
}

fn db_cost(expected: &Database, actual: &Database) -> i64 {
    let mut total = 0;
    for fact in actual.all_facts() {
        if !expected.contains(fact) {
            total += 1;
        }
    }
    for fact in expected.all_facts() {
        if !actual.contains(fact) {
            total += 2;
        }
    }
    return total;
}

impl StepProblem {
    fn rescore(&self, individual: &mut DiagramIndividual, start: Option<NodeIndex>) {
        let mut fitness = 0;
        for ((input, output), eval) in self.samples
            .iter()
            .map(|&(ref i, ref o)| (i, o))
            .zip(individual.evaluations.iter_mut())
        {
            if let Some(result) = if let Some(start) = start {
                eval.rerun_from(&individual.diagram, input, &[start], self.num_registers)
            } else {
                eval.rerun_from(&individual.diagram, input, &[], self.num_registers)
            } {
                *eval = result;
            }
            fitness -= db_cost(output, &eval.total_db);
        }
        individual.fitness = fitness;
    }

    fn mutate_and_rescore<R: Rng>(&self, individual: &mut DiagramIndividual, rng: &mut R) -> bool {
        let context = UniformMutationContext::new(
            self.space.num_nodes,
            self.space.num_terms,
            self.space.num_registers,
            self.frame.values.len() as u64,
            self.frame.num_terms_for_predicate.len() as u64,
            &self.frame,
            &self.space,
        );
        let mutation =
            context.gen_mutation(&individual.diagram, &mut individual.mutation_state, rng);
        if let Some(MutationResult {
            phenotype_could_have_changed,
            node_to_restart,
        }) = apply_mutation(
            &mut individual.diagram,
            mutation,
            &mut individual.mutation_state,
        ) {
            if phenotype_could_have_changed {
                let original_fitness = individual.fitness;
                self.rescore(individual, node_to_restart);
                return individual.fitness != original_fitness;
            }
        }
        return false;
    }
}

impl Problem for StepProblem {
    type Individual = DiagramIndividual;

    fn initialize<R>(&self, count: usize, _rng: &mut R) -> Vec<Self::Individual>
    where
        R: Rng,
    {
        (0..count)
            .map(|_| {
                DiagramIndividual::blank(
                    self.samples.len(),
                    self.num_registers,
                    self.num_nodes,
                    self.num_0_terms,
                )
            })
            .collect()
    }

    fn mutate<R>(&self, individual: &mut Self::Individual, rng: &mut R) -> bool
    where
        R: Rng,
    {
        self.mutate_and_rescore(individual, rng)
    }

    fn compare<R>(
        &self,
        a: &Self::Individual,
        b: &Self::Individual,
        _rng: &mut R,
    ) -> Option<Ordering>
    where
        R: Rng,
    {
        a.fitness.partial_cmp(&b.fitness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::database_literal;
    use evolution_strategies::{Engine, Strategy};
    use predicate::Predicate;
    use rand::SeedableRng;
    use rand::XorShiftRng;
    use value::Value;

    fn evolve_simple_copy() {
        let rng = XorShiftRng::from_seed([0xba, 0xeb, 0xae, 0xee]);
        let problem = StepProblem {
            samples: vec![
                (
                    database_literal(vec![(Predicate(0), vec![Value::Symbol(0)])]),
                    database_literal(vec![(Predicate(1), vec![Value::Symbol(0)])]),
                ),
                (
                    database_literal(vec![(Predicate(0), vec![Value::Symbol(1)])]),
                    database_literal(vec![(Predicate(1), vec![Value::Symbol(1)])]),
                ),
                (
                    database_literal(vec![(Predicate(0), vec![Value::Symbol(2)])]),
                    database_literal(vec![(Predicate(1), vec![Value::Symbol(2)])]),
                ),
            ],
            frame: Frame {
                values: [Value::Symbol(0), Value::Symbol(1), Value::Symbol(2)]
                    .iter()
                    .cloned()
                    .collect(),
                num_terms_for_predicate: [(Predicate(0), 1), (Predicate(1), 0)]
                    .iter()
                    .cloned()
                    .collect(),
            },
            space: DiagramSpace {
                num_nodes: 3,
                num_terms: 1,
                num_registers: 1,
            },
            num_registers: 1,
            num_nodes: 2,
            num_0_terms: 1,
        };
        let strategy = Strategy::MuLambda {
            mu: 100,
            lambda: 200,
        };
        let mut engine = Engine::new(problem, strategy, rng);
        for i in 0..50 {
            if i % 10 == 0 {
                let fitest = engine.fitest();
                println!("fitest = {:#?}", fitest.diagram);
                println!(
                    "fitest total_dbs = {:#?}",
                    fitest
                        .evaluations
                        .iter()
                        .map(|e| &e.total_db)
                        .collect::<Vec<_>>()
                );
                println!("fitness of fitest = {}", fitest.fitness);
                println!("generation = {}", i);
            }
            engine.run_generation();
        }
        assert_eq!(engine.fitest().fitness, 0);
    }
}
