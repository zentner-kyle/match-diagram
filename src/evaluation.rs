use std::iter;

use database::Database;
use diagram::{Diagram, MatchTermConstraint, Node, OutputTerm};
use fact::Fact;
use fixgraph::NodeIndex;
use registers::{RegisterFile, RegisterSet};
use simple_query::{SimpleQuery, SimpleQueryTerm};
use value::Value;

enum PropagateOutput {
    Registers(RegisterSet, RegisterSet),
    Database(Database),
}

fn propagate<D: Diagram>(
    diagram: &D,
    node: NodeIndex,
    database: &Database,
    registers: &RegisterSet,
) -> PropagateOutput {
    match *diagram.get_node(node) {
        Node::Match {
            predicate,
            ref terms,
        } => {
            let mut match_set = RegisterSet::new(registers.num_registers());
            let mut refute_set = RegisterSet::new(registers.num_registers());
            for register_file in registers.iter() {
                for fact in database.facts_for_predicate(predicate) {
                    let mut result_registers = register_file.clone();
                    let mut refuted = false;
                    for (term, value) in terms.iter().zip(fact.values) {
                        match term.constraint {
                            MatchTermConstraint::Free => {}
                            MatchTermConstraint::Constant(ref v) => if v != value {
                                refuted = true;
                            },
                            MatchTermConstraint::Register(reg) => {
                                if register_file[reg].as_ref() != Some(value) {
                                    refuted = true;
                                }
                            }
                        }
                        if let Some(target) = term.target {
                            result_registers[target] = Some(value.clone());
                        }
                    }
                    if refuted {
                        refute_set.push(result_registers);
                    } else {
                        match_set.push(result_registers);
                    }
                }
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

#[derive(Clone, Debug)]
pub struct Evaluation {
    pub input_sets: Vec<RegisterSet>,
    pub output_sets: Vec<(RegisterSet, RegisterSet)>,
    pub output_dbs: Vec<Option<Database>>,
    pub total_db: Database,
}

impl Evaluation {
    pub fn new<D: Diagram>(diagram: &D, num_registers: usize) -> Self {
        let input_sets: Vec<RegisterSet> = iter::repeat(RegisterSet::new(num_registers))
            .take(diagram.len())
            .collect();
        let output_sets: Vec<(RegisterSet, RegisterSet)> = iter::repeat((
            RegisterSet::new(num_registers),
            RegisterSet::new(num_registers),
        )).take(diagram.len())
            .collect();
        let output_dbs: Vec<_> = iter::repeat(None).take(diagram.len()).collect();
        let total_db = Database::new();
        Evaluation {
            input_sets,
            output_sets,
            output_dbs,
            total_db,
        }
    }

    pub fn run<D: Diagram>(diagram: &D, input: &Database, num_registers: usize) -> Self {
        let mut eval = Self::new(diagram, num_registers);
        let root = diagram.get_root();
        eval.input_sets[root.0].push(RegisterFile::new(num_registers));
        eval.start_at(diagram, root, input);
        eval
    }

    pub fn start_at<D: Diagram>(&mut self, diagram: &D, node: NodeIndex, input: &Database) {
        let mut pending_nodes = vec![node];
        while let Some(node_index) = pending_nodes.pop() {
            match propagate(diagram, node_index, input, &self.input_sets[node_index.0]) {
                PropagateOutput::Registers(match_set, refute_set) => {
                    let (ref mut old_match_set, ref mut old_refute_set) =
                        self.output_sets[node_index.0];
                    if *old_match_set != match_set {
                        for registers in match_set.iter() {
                            old_match_set.push(registers.clone());
                        }
                        if let Some(match_node) = diagram.get_on_match(node_index) {
                            let input_sets = &mut self.input_sets[match_node.0];
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
                            let input_sets = &mut self.input_sets[refute_node.0];
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
                    self.output_dbs[node_index.0] = Some(db);
                }
            }
        }
        for db in self.output_dbs.iter().filter_map(|db| db.as_ref()) {
            for fact in db.all_facts() {
                self.total_db.insert_fact(fact);
            }
        }
    }
}
