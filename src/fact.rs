use predicate::Predicate;
use value::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact<'a> {
    pub predicate: Predicate,
    pub values: &'a [Value],
}
