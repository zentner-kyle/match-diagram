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

    pub fn all_facts(&self) -> AllFactIter {
        AllFactIter {
            tables_iter: self.tables.iter(),
            current_table: None,
            row: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AllFactIter<'a> {
    tables_iter: hash_map::Iter<'a, Predicate, Table>,
    current_table: Option<(Predicate, &'a Table)>,
    row: usize,
}

impl<'a> Iterator for AllFactIter<'a> {
    type Item = Fact<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((predicate, table)) = self.current_table {
            if self.row < table.num_rows() {
                let row = table.row(self.row);
                self.row += 1;
                return Some(Fact {
                    predicate,
                    values: row,
                });
            }
        };
        if let Some((&predicate, table)) = self.tables_iter.next() {
            self.current_table = Some((predicate, table));
            self.row = 0;
            return self.next();
        } else {
            return None;
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

    fn insert_symbols_run_query_expect_rows(
        symbols: &[&[u64]],
        query: SimpleQuery,
        expected: &[usize],
    ) {
        let predicate = query.predicate;
        let values: Vec<Vec<_>> = symbols
            .iter()
            .map(|row| row.iter().map(|&i| Value::Symbol(i)).collect())
            .collect();
        let facts: Vec<_> = values
            .iter()
            .map(|vs| Fact {
                predicate,
                values: vs,
            })
            .collect();
        let expected: Vec<_> = expected
            .iter()
            .map(|&i| Fact {
                predicate,
                values: &values[i],
            })
            .collect();
        insert_facts_run_query_expect_facts(&facts, query, &expected);
    }

    fn insert_facts_run_query_expect_facts(input: &[Fact], query: SimpleQuery, expected: &[Fact]) {
        let mut db = Database::new();
        for fact in input {
            db.insert_fact(fact.clone());
        }
        let mut iter = db.simple_query(query);
        for fact in expected {
            assert_eq!(Some(fact.clone()), iter.next());
        }
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn can_query_single_fact_database() {
        let predicate = predicate::Predicate(0);
        let values = [[Value::Symbol(1), Value::Symbol(2)]];
        let facts: Vec<_> = values
            .iter()
            .map(|vs| Fact {
                predicate,
                values: vs,
            })
            .collect();
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
        let facts: Vec<_> = values
            .iter()
            .map(|vs| Fact {
                predicate,
                values: vs,
            })
            .collect();
        let terms = &[SimpleQueryTerm::Free, SimpleQueryTerm::Free];
        let query = SimpleQuery { predicate, terms };
        insert_facts_run_query_expect_facts(&facts, query, &facts);
    }

    #[test]
    fn can_filter() {
        let predicate = predicate::Predicate(0);
        let symbols: Vec<&[u64]> = [[1, 2], [2, 1], [1, 3], [2, 3]]
            .iter()
            .map(|s| &s[..])
            .collect();
        let terms = &[
            SimpleQueryTerm::Constant {
                value: &Value::Symbol(1),
            },
            SimpleQueryTerm::Free,
        ];
        let query = SimpleQuery { predicate, terms };
        insert_symbols_run_query_expect_rows(&symbols, query, &[0, 2]);
    }

    #[test]
    fn can_filter_multiple_columns() {
        let predicate = predicate::Predicate(0);
        let symbols: Vec<&[u64]> = [[1, 2, 1], [2, 2, 2], [1, 1, 3], [2, 2, 4], [1, 2, 5]]
            .iter()
            .map(|s| &s[..])
            .collect();
        let terms = &[
            SimpleQueryTerm::Constant {
                value: &Value::Symbol(1),
            },
            SimpleQueryTerm::Constant {
                value: &Value::Symbol(2),
            },
            SimpleQueryTerm::Free,
        ];
        let query = SimpleQuery { predicate, terms };
        insert_symbols_run_query_expect_rows(&symbols, query, &[0, 4]);
    }
}
