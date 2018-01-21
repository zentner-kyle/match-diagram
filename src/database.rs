use std::collections::HashMap;
use std::collections::hash_map;

use fact::Fact;
use index::{Index, IndexIter};
use predicate::Predicate;
use simple_query::{SimpleQuery, SimpleQueryTerm};
use table;
use table::Table;
use value::Value;
use weight::Weight;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Database {
    tables: HashMap<Predicate, Table>,
}

impl Database {
    pub fn new() -> Self {
        Database {
            tables: HashMap::new(),
        }
    }

    pub fn insert_fact<'a, 'b>(&'a mut self, fact: Fact<'b>) {
        self.insert_fact_with_weight(fact, Weight(1));
    }

    pub fn insert_fact_with_weight<'a, 'b>(&'a mut self, fact: Fact<'b>, weight: Weight) {
        match self.tables.entry(fact.predicate) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().push(fact.values, weight);
            }
            hash_map::Entry::Vacant(entry) => {
                let mut table = Table::new(fact.values.len());
                table.push(fact.values, weight);
                entry.insert(table);
            }
        };
    }

    pub fn simple_query<'a, 'b, 'c>(
        &'a self,
        query: SimpleQuery<'b, 'c>,
    ) -> SimpleQueryIter<'a, 'b, 'c> {
        SimpleQueryIter {
            predicate_iter: self.facts_for_predicate(query.predicate),
            query,
        }
    }

    pub fn facts_for_predicate(&self, predicate: Predicate) -> PredicateIter {
        PredicateIter {
            predicate,
            inner: self.tables.get(&predicate).map(|t| t.iter()),
        }
    }

    pub fn all_facts(&self) -> AllFactIter {
        AllFactIter {
            inner: self.weighted_facts(),
        }
    }

    pub fn weighted_facts(&self) -> WeightedFacts {
        WeightedFacts {
            tables_iter: self.tables.iter(),
            current_table: None,
            row: 0,
        }
    }

    pub fn contains(&self, fact: Fact) -> bool {
        if let Some(table) = self.tables.get(&fact.predicate) {
            for row in table.iter() {
                if row == fact.values {
                    return true;
                }
            }
        }
        return false;
    }

    pub fn weight(&self, fact: Fact) -> Weight {
        let mut total = 0;
        if let Some(table) = self.tables.get(&fact.predicate) {
            for (row, weight) in table.weighted_rows() {
                if row == fact.values {
                    total += weight.0;
                }
            }
        }
        return Weight(total);
    }
}

#[derive(Clone, Debug)]
pub struct PredicateIter<'a> {
    predicate: Predicate,
    inner: Option<table::Iter<'a>>,
}

impl<'a> Iterator for PredicateIter<'a> {
    type Item = Fact<'a>;

    fn next(&mut self) -> Option<Fact<'a>> {
        if let Some(ref mut iter) = self.inner {
            if let Some(values) = iter.next() {
                return Some(Fact {
                    predicate: self.predicate,
                    values,
                });
            }
        }
        return None;
    }
}

#[derive(Clone, Debug)]
pub struct AllFactIter<'a> {
    inner: WeightedFacts<'a>,
}

impl<'a> Iterator for AllFactIter<'a> {
    type Item = Fact<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(f, _)| f)
    }
}

#[derive(Clone, Debug)]
pub struct WeightedFacts<'a> {
    tables_iter: hash_map::Iter<'a, Predicate, Table>,
    current_table: Option<(Predicate, &'a Table)>,
    row: usize,
}

impl<'a> Iterator for WeightedFacts<'a> {
    type Item = (Fact<'a>, Weight);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((predicate, table)) = self.current_table {
            if self.row < table.num_rows() {
                let row = table.row(self.row);
                let weight = table.weight(self.row);
                self.row += 1;
                return Some((
                    Fact {
                        predicate,
                        values: row,
                    },
                    weight,
                ));
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
pub struct SimpleQueryIter<'a, 'b, 'c: 'b> {
    predicate_iter: PredicateIter<'a>,
    query: SimpleQuery<'b, 'c>,
}

impl<'a, 'b, 'c> Iterator for SimpleQueryIter<'a, 'b, 'c> {
    type Item = Fact<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(fact) = self.predicate_iter.next() {
            if self.query.matches(fact) {
                return Some(fact);
            }
        }
        return None;
    }
}

pub fn database_literal(data: Vec<(Predicate, Vec<Value>)>) -> Database {
    let mut db = Database::new();
    for &(predicate, ref values) in data.iter() {
        let fact = Fact { predicate, values };
        db.insert_fact(fact);
    }
    return db;
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
