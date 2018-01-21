use value::Value;
use weight::Weight;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    num_columns: usize,
    num_rows: usize,
    values: Vec<Value>,
    row_weights: Vec<Weight>,
}

impl Table {
    pub fn new(num_columns: usize) -> Self {
        Table {
            num_columns,
            num_rows: 0,
            values: Vec::new(),
            row_weights: Vec::new(),
        }
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn weight(&self, row: usize) -> Weight {
        self.row_weights[row]
    }

    pub fn weight_mut(&mut self, row: usize) -> &mut Weight {
        &mut self.row_weights[row]
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

    pub fn push(&mut self, row: &[Value], weight: Weight) -> usize {
        assert!(row.len() == self.num_columns);
        self.values.extend_from_slice(row);
        self.row_weights.push(weight);
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

    pub fn weighted_rows(&self) -> WeightedRows {
        WeightedRows {
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

#[derive(Clone, Debug)]
pub struct WeightedRows<'a> {
    table: &'a Table,
    row: usize,
}

impl<'a> Iterator for WeightedRows<'a> {
    type Item = (&'a [Value], Weight);

    fn next(&mut self) -> Option<Self::Item> {
        if self.row < self.table.num_rows() {
            let values = self.table.row(self.row);
            let weight = self.table.weight(self.row);
            self.row += 1;
            Some((values, weight))
        } else {
            None
        }
    }
}
