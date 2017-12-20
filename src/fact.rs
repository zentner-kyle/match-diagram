use predicate::Predicate;
use value::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fact<'a> {
    pub predicate: Predicate,
    pub values: &'a [Value],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OwnedFact {
    pub predicate: Predicate,
    pub values: Vec<Value>,
}
