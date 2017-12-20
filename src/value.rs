#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Value {
    Symbol(u64),
    Nil,
}
