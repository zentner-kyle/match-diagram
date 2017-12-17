use predicate::Predicate;
use value::Value;

#[derive(Clone, Debug)]
pub enum SimpleQueryTerm<'a> {
    Constant { value: &'a Value },
    Free,
}

#[derive(Clone, Debug)]
pub struct SimpleQuery<'a, 'b: 'a> {
    pub predicate: Predicate,
    pub terms: &'a [SimpleQueryTerm<'b>],
}
