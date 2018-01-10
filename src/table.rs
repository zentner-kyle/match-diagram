use value::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    num_columns: usize,
    num_rows: usize,
    values: Vec<Value>,
}

impl Table {
    pub fn new(num_columns: usize) -> Self {
        Table {
            num_columns,
            num_rows: 0,
            values: Vec::new(),
        }
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn row(&self, row: usize) -> &[Value] {
        let start = self.num_columns * row;
        let end = start + self.num_columns;
        &self.values[start..end]
    }

    pub fn row_mut(&mut self, row: usize) -> &mut [Value] {
        let start = self.num_columns * row;
        let end = start + self.num_columns;
        &mut self.values[start..end]
    }

    pub fn push(&mut self, row: &[Value]) -> usize {
        assert!(row.len() == self.num_columns);
        self.values.extend_from_slice(row);
        let result = self.num_rows;
        self.num_rows += 1;
        result
    }

    pub fn iter(&self) -> Iter {
        Iter {
            table: self,
            row: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    table: &'a Table,
    row: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a [Value];

    fn next(&mut self) -> Option<Self::Item> {
        if self.row < self.table.num_rows() {
            let result = self.table.row(self.row);
            self.row += 1;
            Some(result)
        } else {
            None
        }
    }
}
