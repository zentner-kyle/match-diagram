use std::collections::HashSet;
use std::iter;

use database::Database;
use diagram::{EdgeGroup, MatchTerm, MatchTermConstraint, MultiDiagram, Node, OutputTerm};
use fact::Fact;
use node_index::NodeIndex;
use predicate::Predicate;
use registers::{RegisterFile, RegisterSet};
use simple_query::{SimpleQuery, SimpleQueryTerm};
use value::Value;
use weight::Weight;

#[derive(Clone, Debug)]
struct NodeState {
    input: RegisterSet,
    output: Option<NodeOutputState>,
}

impl NodeState {
    /**
     * Returns whether a new state was added to the output.
     */
    fn merge_output(&mut self, output: NodeOutputState) -> bool {
        let mut found_new_state = false;
        match (&mut self.output, output) {
            (
                &mut Some(NodeOutputState::Output { db: ref mut old_db }),
                NodeOutputState::Output { db: ref new_db },
            ) => for (fact, w) in new_db.weighted_facts() {
                old_db.insert_fact_with_weight(fact, w);
            },
            (
                &mut Some(NodeOutputState::Match {
                    matches: ref mut old_matches,
                    refutes: ref mut old_refutes,
                }),
                NodeOutputState::Match {
                    matches: ref new_matches,
                    refutes: ref new_refutes,
                },
            ) => {
                for (r, w, d) in new_matches.iter() {
                    found_new_state |= old_matches.push(r.clone(), w, d);
                }
                for (r, w, d) in new_refutes.iter() {
                    found_new_state |= old_refutes.push(r.clone(), w, d);
                }
            }
            (self_output @ &mut None, output) => {
                *self_output = Some(output);
                found_new_state = true;
            }
            _ => {
                panic!("Node should not have changed type");
            }
        }
        return found_new_state;
    }
}

#[derive(Clone, Debug)]
enum NodeOutputState {
    Match {
        matches: RegisterSet,
        refutes: RegisterSet,
    },
    Output {
        db: Database,
    },
}

/**
 * Return whether a new state was added to one of the outputs.
 */
fn propagate_match_node_into_output(
    predicate: Predicate,
    terms: &[MatchTerm],
    database: &Database,
    register_file: &RegisterFile,
    weight: Weight,
    input_depth: usize,
    matches: &mut RegisterSet,
    refutes: &mut RegisterSet,
) -> bool {
    let mut found_new_state = false;
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
            found_new_state |= refutes.push(result_registers, weight, input_depth + 1);
        } else {
            found_new_state |= matches.push(result_registers, weight, input_depth + 1);
        }
    }
    return found_new_state;
}

fn propagate_output_node_into_output(
    predicate: Predicate,
    terms: &[OutputTerm],
    register_file: &RegisterFile,
    weight: Weight,
    db: &mut Database,
) {
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
    db.insert_fact_with_weight(
        Fact {
            predicate,
            values: &values[..],
        },
        weight,
    );
}

fn propagate<D: MultiDiagram>(
    diagram: &D,
    node: NodeIndex,
    database: &Database,
    registers: &RegisterSet,
    max_depth: Option<usize>,
) -> NodeOutputState {
    match *diagram.get_node(node) {
        Node::Match {
            predicate,
            ref terms,
        } => {
            let mut matches = RegisterSet::new(registers.num_registers());
            let mut refutes = RegisterSet::new(registers.num_registers());
            for (register_file, weight, depth) in registers.iter() {
                if max_depth.map(|max_depth| depth < max_depth).unwrap_or(true) {
                    propagate_match_node_into_output(
                        predicate,
                        terms,
                        database,
                        register_file,
                        weight,
                        depth,
                        &mut matches,
                        &mut refutes,
                    );
                }
            }
            NodeOutputState::Match { matches, refutes }
        }
        Node::Output {
            predicate,
            ref terms,
        } => {
            let mut db = Database::new();
            for (register_file, weight, _) in registers.iter() {
                propagate_output_node_into_output(predicate, terms, register_file, weight, &mut db);
            }
            NodeOutputState::Output { db }
        }
    }
}

const DEFAULT_MAX_DEPTH: usize = 8;

#[derive(Clone, Debug)]
pub struct Evaluation {
    states: Vec<NodeState>,
    max_depth: usize,
    pub total_db: Database,
}

impl Evaluation {
    pub fn new() -> Self {
        Evaluation {
            states: Vec::new(),
            max_depth: DEFAULT_MAX_DEPTH,
            total_db: Database::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Evaluation {
            states: Vec::with_capacity(cap),
            max_depth: DEFAULT_MAX_DEPTH,
            total_db: Database::new(),
        }
    }

    pub fn eval<D: MultiDiagram>(diagram: &D, input: &Database, num_registers: usize) -> Self {
        let mut eval = Self::new();
        eval.evaluate_recursively(diagram, input, num_registers);
        eval
    }

    pub fn evaluate_recursively<D: MultiDiagram>(
        &mut self,
        diagram: &D,
        input: &Database,
        num_registers: usize,
    ) {
        self.grow(diagram.len(), num_registers);
        for &root in diagram.get_group(EdgeGroup::Roots) {
            self.evaluate_recursively_inner(
                diagram,
                input,
                root,
                &RegisterFile::new(num_registers),
                Weight(1),
                0,
            );
        }
    }

    fn recurse_on_group<D: MultiDiagram>(
        &mut self,
        diagram: &D,
        input: &Database,
        group: &[NodeIndex],
        register_set: &RegisterSet,
        weight: Weight,
    ) {
        for match_node in group {
            for (regs, w, depth) in register_set.iter() {
                self.evaluate_recursively_inner(
                    diagram,
                    input,
                    *match_node,
                    regs,
                    Weight(weight.0 * w.0),
                    depth,
                );
            }
        }
    }

    fn evaluate_recursively_inner<D: MultiDiagram>(
        &mut self,
        diagram: &D,
        input: &Database,
        node: NodeIndex,
        registers: &RegisterFile,
        weight: Weight,
        depth: usize,
    ) {
        if node.0 >= self.states.len() {
            return;
        }
        self.states[node.0]
            .input
            .push(registers.clone(), weight, depth);
        match *diagram.get_node(node) {
            Node::Match {
                predicate,
                ref terms,
            } => {
                let mut matches = RegisterSet::new(registers.len());
                let mut refutes = RegisterSet::new(registers.len());
                if propagate_match_node_into_output(
                    predicate,
                    terms,
                    input,
                    registers,
                    weight,
                    depth,
                    &mut matches,
                    &mut refutes,
                ) && depth < self.max_depth
                {
                    self.recurse_on_group(
                        diagram,
                        input,
                        diagram.get_group(EdgeGroup::MatchTargets(node)),
                        &matches,
                        weight,
                    );
                    self.recurse_on_group(
                        diagram,
                        input,
                        diagram.get_group(EdgeGroup::RefuteTargets(node)),
                        &refutes,
                        weight,
                    );
                }
                self.states[node.0].merge_output(NodeOutputState::Match { matches, refutes });
            }
            Node::Output {
                predicate,
                ref terms,
            } => {
                if let NodeOutputState::Output { ref mut db } = *self.states[node.0]
                    .output
                    .get_or_insert_with(|| NodeOutputState::Output {
                        db: Database::new(),
                    }) {
                    propagate_output_node_into_output(predicate, terms, registers, weight, db);
                } else {
                    panic!("node changed type?");
                }
            }
        }
    }

    fn grow(&mut self, num_nodes: usize, num_registers: usize) {
        for _ in self.states.len()..num_nodes {
            self.states.push(NodeState {
                input: RegisterSet::new(num_registers),
                output: None,
            });
        }
    }

    pub fn run_multi<D: MultiDiagram>(diagram: &D, input: &Database, num_registers: usize) -> Self {
        let mut eval = Self::new();
        eval.grow(diagram.len(), num_registers);
        for root in diagram.get_group(EdgeGroup::Roots) {
            if root.0 >= diagram.len() {
                continue;
            }
            eval.states[root.0]
                .input
                .push(RegisterFile::new(num_registers), Weight(1), 0);
        }
        let pending: Vec<(NodeIndex, RegisterSet)> = diagram
            .get_group(EdgeGroup::Roots)
            .iter()
            .filter_map(|n| {
                let mut regs = RegisterSet::new(num_registers);
                regs.push(RegisterFile::new(num_registers), Weight(1), 0);
                if n.0 < diagram.len() {
                    Some((*n, regs))
                } else {
                    None
                }
            })
            .collect();
        eval.run_pending(diagram, input, pending);
        eval.build_total_db();
        eval
    }

    pub fn run_pending<D: MultiDiagram>(
        &mut self,
        diagram: &D,
        input: &Database,
        mut pending: Vec<(NodeIndex, RegisterSet)>,
    ) {
        while let Some((node, regs)) = pending.pop() {
            for (r, w, d) in regs.iter() {
                self.states[node.0].input.push(r.clone(), w, d);
            }
            let output = propagate(diagram, node, input, &regs, Some(self.max_depth));
            if self.states[node.0].merge_output(output.clone()) {
                if let NodeOutputState::Match {
                    ref matches,
                    ref refutes,
                } = output
                {
                    for n in diagram.get_group(EdgeGroup::MatchTargets(node)) {
                        pending.push((*n, matches.clone()));
                    }
                    for n in diagram.get_group(EdgeGroup::RefuteTargets(node)) {
                        pending.push((*n, refutes.clone()));
                    }
                };
            }
        }
    }

    pub fn build_total_db(&mut self) {
        for db in self.states.iter().filter_map(|state| {
            if let &Some(NodeOutputState::Output { ref db }) = &state.output {
                Some(db)
            } else {
                None
            }
        }) {
            for fact in db.all_facts() {
                self.total_db.insert_fact(fact);
            }
        }
    }

    pub fn rerun_from<D: MultiDiagram>(
        &self,
        diagram: &D,
        input: &Database,
        start: &[NodeIndex],
        num_registers: usize,
    ) -> Option<Self> {
        // Invalidate the transitive closure from starting nodes.
        // If the transitive closure of the starting nodes includes any of the starting nodes,
        // restart from the root.
        let start_set: HashSet<NodeIndex> = start.iter().cloned().collect();
        let mut eval = self.clone();
        eval.grow(diagram.len(), num_registers);
        eval.total_db = Database::new();
        let mut to_invalidate = start.to_owned();
        let mut invalidated = HashSet::new();
        while let Some(node) = to_invalidate.pop() {
            if invalidated.contains(&node) {
                continue;
            }
            invalidated.insert(node);
            eval.states[node.0] = NodeState {
                input: RegisterSet::new(num_registers),
                output: None,
            };
            for n in diagram
                .get_group(EdgeGroup::MatchTargets(node))
                .iter()
                .chain(diagram.get_group(EdgeGroup::RefuteTargets(node)).iter())
            {
                if start_set.contains(n) {
                    return Some(Evaluation::run_multi(diagram, input, num_registers));
                }
                to_invalidate.push(*n);
            }
        }
        let mut pending = Vec::with_capacity(start_set.len());
        let roots: HashSet<NodeIndex> = diagram
            .get_group(EdgeGroup::Roots)
            .iter()
            .cloned()
            .collect();
        for node in start {
            let input = &mut eval.states[node.0].input;
            for source in diagram.get_group(EdgeGroup::MatchSources(*node)) {
                if source.0 < self.states.len() {
                    if let Some(NodeOutputState::Match { ref matches, .. }) =
                        self.states[source.0].output
                    {
                        for (r, w, d) in matches.iter() {
                            input.push(r.clone(), w, d);
                        }
                    }
                }
            }
            for source in diagram.get_group(EdgeGroup::RefuteSources(*node)) {
                if source.0 < self.states.len() {
                    if let Some(NodeOutputState::Match { ref refutes, .. }) =
                        self.states[source.0].output
                    {
                        for (r, w, d) in refutes.iter() {
                            input.push(r.clone(), w, d);
                        }
                    }
                }
            }
            if roots.contains(node) {
                input.push(RegisterFile::new(num_registers), Weight(1), 0);
            }
            pending.push((*node, input.clone()));
        }
        eval.run_pending(diagram, input, pending);
        eval.build_total_db();
        return Some(eval);
    }
}
