use fact::Fact;
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

impl<'a, 'b: 'a> SimpleQuery<'a, 'b> {
    pub fn matches(&self, fact: Fact) -> bool {
        self.predicate == fact.predicate
            && self.terms
                .iter()
                .zip(fact.values.iter())
                .all(|(term, ref v)| match *term {
                    SimpleQueryTerm::Constant { ref value } => v == value,
                    SimpleQueryTerm::Free => true,
                })
    }
}
