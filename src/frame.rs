use std::collections::{HashMap, HashSet};

use predicate::Predicate;
use value::Value;

#[derive(Debug, Clone)]
pub struct Frame {
    pub values: HashSet<Value>,
    pub num_terms_for_predicate: HashMap<Predicate, usize>,
}
