use std::collections::HashMap;
use std::collections::hash_map;

use diagram::{Diagram, MultiDiagram, Node};
use node_index::NodeIndex;
use predicate::Predicate;

#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub index: NodeIndex,
    pub defined: bool,
}

#[derive(Clone, Debug)]
pub struct Context {
    pub num_terms_for_predicate: HashMap<Predicate, usize>,
    pub predicate_name_to_predicate: HashMap<String, Predicate>,
    pub node_name_to_info: HashMap<String, NodeInfo>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            num_terms_for_predicate: HashMap::new(),
            predicate_name_to_predicate: HashMap::new(),
            node_name_to_info: HashMap::new(),
        }
    }

    pub fn check_num_terms_for_predicate(&mut self, predicate: Predicate, num_terms: usize) {
        match self.num_terms_for_predicate.entry(predicate) {
            hash_map::Entry::Occupied(entry) => {
                if *entry.get() != num_terms {
                    panic!("Wrong number of terms for predicate");
                }
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(num_terms);
            }
        }
    }

    pub fn get_num_terms_for_predicate(&self, predicate: Predicate) -> Option<usize> {
        self.num_terms_for_predicate.get(&predicate).cloned()
    }

    pub fn reserve_node_name(&mut self, name: &str, diagram: &mut Diagram) -> NodeInfo {
        if self.node_name_to_info.contains_key(name) {
            self.node_name_to_info.get(name).unwrap().clone()
        } else {
            let node = Node::Match {
                predicate: Predicate(0),
                terms: Vec::new(),
            };
            let index = diagram.insert_node(node);
            let info = NodeInfo {
                index,
                defined: false,
            };
            self.node_name_to_info.insert(name.to_owned(), info.clone());
            info
        }
    }

    pub fn reserve_predicate(&mut self, name: &str) -> Predicate {
        let next_predicate = Predicate(self.predicate_name_to_predicate.len() as u64);
        if self.predicate_name_to_predicate.contains_key(name) {
            *self.predicate_name_to_predicate.get(name).unwrap()
        } else {
            self.predicate_name_to_predicate
                .insert(name.to_owned(), next_predicate);
            next_predicate
        }
    }
}
