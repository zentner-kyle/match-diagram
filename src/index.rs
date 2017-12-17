use std::iter;
use std::slice;

use value::Value;

#[derive(Clone, Debug)]
pub struct Index {
    column: usize,
    value: Value,
    row_indices: Vec<usize>,
}

impl Index {
    pub fn new(column: usize, value: Value) -> Self {
        Index {
            column,
            value,
            row_indices: Vec::new(),
        }
    }

    pub fn add_row(&mut self, row: &[Value], row_index: usize) -> bool {
        if row[self.column] == self.value {
            assert!(
                self.row_indices
                    .last()
                    .map(|&prev| prev < row_index)
                    .unwrap_or(true)
            );
            self.row_indices.push(row_index);
            true
        } else {
            false
        }
    }

    pub fn iter(&self) -> IndexIter {
        IndexIter {
            inner: self.row_indices.iter().peekable(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndexIter<'a> {
    inner: iter::Peekable<slice::Iter<'a, usize>>,
}

impl<'a> Iterator for IndexIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().cloned()
    }
}

impl<'a> IndexIter<'a> {
    pub fn peek(&mut self) -> Option<usize> {
        self.inner.peek().map(|&&r| r)
    }

    pub fn jump_to_row(&mut self, row: usize) -> bool {
        while let Some(next_row) = self.peek() {
            if row == next_row {
                self.next();
                return true;
            } else if row < next_row {
                return false;
            }
        }
        return false;
    }
}
