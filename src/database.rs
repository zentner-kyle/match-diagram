use std::collections::HashMap;
use std::collections::hash_map;

use fact::Fact;
use index::{Index, IndexIter};
use predicate::Predicate;
use simple_query::{SimpleQuery, SimpleQueryTerm};
use table::Table;
use value::Value;

#[derive(Clone, Debug)]
pub struct Database {
    tables: HashMap<Predicate, Table>,
    indices: HashMap<(Predicate, usize, Value), Index>,
}

#[derive(Clone, Debug)]
pub struct FreeColumnIter {
    next: usize,
    num_rows: usize,
}

impl Iterator for FreeColumnIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.next;
        if result == self.num_rows {
            None
        } else {
            self.next += 1;
            Some(result)
        }
    }
}

impl FreeColumnIter {
    fn new(table: &Table) -> Self {
        FreeColumnIter {
            next: 0,
            num_rows: table.num_rows(),
        }
    }

    fn peek(&mut self) -> Option<usize> {
        if self.next == self.num_rows {
            None
        } else {
            Some(self.next)
        }
    }

    fn jump_to_row(&mut self, row: usize) -> bool {
        if row < self.num_rows {
            self.next = row + 1;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug)]
enum SimpleQueryIndexIter<'a> {
    Index(IndexIter<'a>),
    Free(FreeColumnIter),
}

impl<'a> SimpleQueryIndexIter<'a> {
    fn peek(&mut self) -> Option<usize> {
        match *self {
            SimpleQueryIndexIter::Index(ref mut iter) => iter.peek(),
            SimpleQueryIndexIter::Free(ref mut iter) => iter.peek(),
        }
    }

    fn jump_to_row(&mut self, row: usize) -> bool {
        match *self {
            SimpleQueryIndexIter::Index(ref mut iter) => iter.jump_to_row(row),
            SimpleQueryIndexIter::Free(ref mut iter) => iter.jump_to_row(row),
        }
    }
}

impl Database {
    pub fn new() -> Self {
        Database {
            tables: HashMap::new(),
            indices: HashMap::new(),
        }
    }

    pub fn insert_fact<'a, 'b>(&'a mut self, fact: Fact<'b>) {
        let row_idx = match self.tables.entry(fact.predicate) {
            hash_map::Entry::Occupied(mut entry) => entry.get_mut().push(fact.values),
            hash_map::Entry::Vacant(entry) => {
                let mut table = Table::new(fact.values.len());
                table.push(fact.values);
                entry.insert(table);
                0
            }
        };
        for (column, value) in fact.values.iter().enumerate() {
            let index_key = (fact.predicate, column, value.clone());
            match self.indices.entry(index_key) {
                hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().add_row(fact.values, row_idx);
                }
                hash_map::Entry::Vacant(entry) => {
                    let mut index = Index::new(column, value.clone());
                    index.add_row(fact.values, row_idx);
                    entry.insert(index);
                }
            }
        }
    }

    pub fn simple_query<'a, 'b, 'c>(&'a self, query: SimpleQuery<'b, 'c>) -> SimpleQueryIter<'a> {
        let mut iters: Vec<SimpleQueryIndexIter<'a>> = Vec::with_capacity(query.terms.len());
        let predicate = query.predicate;
        let table = self.tables.get(&predicate).expect("no table for predicate");
        for (i, term) in query.terms.iter().enumerate() {
            if let &SimpleQueryTerm::Constant { value } = term {
                let index_key = (predicate, i, value.clone());
                let index = self.indices
                    .get(&index_key)
                    .expect("Should have an index for everything");
                iters.push(SimpleQueryIndexIter::Index(index.iter()));
            } else {
                iters.push(SimpleQueryIndexIter::Free(FreeColumnIter::new(table)));
            }
        }

        SimpleQueryIter {
            predicate,
            iters,
            table,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimpleQueryIter<'a> {
    predicate: Predicate,
    table: &'a Table,
    iters: Vec<SimpleQueryIndexIter<'a>>,
}

impl<'a> Iterator for SimpleQueryIter<'a> {
    type Item = Fact<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut max_row = 0;
            for iter in &mut self.iters {
                if let Some(next_row) = iter.peek() {
                    if max_row < next_row {
                        max_row = next_row;
                    }
                } else {
                    return None;
                }
            }
            let mut got_row = true;
            for iter in &mut self.iters {
                got_row &= iter.jump_to_row(max_row);
            }
            if got_row {
                return Some(Fact {
                    predicate: self.predicate,
                    values: self.table.row(max_row),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use predicate;
    use simple_query::SimpleQueryTerm;

    fn insert_facts_run_query_expect_facts(input: &[Fact], query: SimpleQuery, expect: &[Fact]) {
        let mut db = Database::new();
        for fact in input {
            db.insert_fact(fact.clone());
        }
        let mut iter = db.simple_query(query);
        for fact in expect {
            assert_eq!(Some(fact.clone()), iter.next());
        }
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn can_query_single_fact_database() {
        let predicate = predicate::Predicate(0);
        let values = [[Value::Symbol(1), Value::Symbol(2)]];
        let facts = [
            Fact {
                predicate,
                values: &values[0],
            },
        ];
        let terms = &[SimpleQueryTerm::Free, SimpleQueryTerm::Free];
        let query = SimpleQuery { predicate, terms };
        insert_facts_run_query_expect_facts(&facts, query, &facts);
    }

    #[test]
    fn can_query_two_fact_database() {
        let predicate = predicate::Predicate(0);
        let values = [
            [Value::Symbol(1), Value::Symbol(2)],
            [Value::Symbol(3), Value::Symbol(4)],
        ];
        let facts = [
            Fact {
                predicate,
                values: &values[0],
            },
            Fact {
                predicate,
                values: &values[1],
            },
        ];
        let terms = &[SimpleQueryTerm::Free, SimpleQueryTerm::Free];
        let query = SimpleQuery { predicate, terms };
        insert_facts_run_query_expect_facts(&facts, query, &facts);
    }

    #[test]
    fn can_filter() {
        let predicate = predicate::Predicate(0);
        let values = [
            [Value::Symbol(1), Value::Symbol(2)],
            [Value::Symbol(2), Value::Symbol(1)],
            [Value::Symbol(1), Value::Symbol(3)],
            [Value::Symbol(2), Value::Symbol(3)],
        ];
        let facts = [
            Fact {
                predicate,
                values: &values[0],
            },
            Fact {
                predicate,
                values: &values[1],
            },
            Fact {
                predicate,
                values: &values[2],
            },
            Fact {
                predicate,
                values: &values[3],
            },
        ];
        let expected = [
            Fact {
                predicate,
                values: &values[0],
            },
            Fact {
                predicate,
                values: &values[2],
            },
        ];
        let terms = &[
            SimpleQueryTerm::Constant {
                value: &Value::Symbol(1),
            },
            SimpleQueryTerm::Free,
        ];
        let query = SimpleQuery { predicate, terms };
        insert_facts_run_query_expect_facts(&facts, query, &expected);
    }
}
