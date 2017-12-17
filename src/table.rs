use value::Value;

#[derive(Clone, Debug)]
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
}
