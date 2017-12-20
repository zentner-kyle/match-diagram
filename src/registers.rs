use std::collections::HashSet;
use std::collections::hash_set;
use std::iter;
use std::ops;

use value::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegisterFile {
    registers: Vec<Option<Value>>,
}

impl RegisterFile {
    pub fn new(size: usize) -> Self {
        RegisterFile {
            registers: iter::repeat(None).take(size).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.registers.len()
    }
}

impl ops::Index<usize> for RegisterFile {
    type Output = Option<Value>;

    fn index(&self, index: usize) -> &Option<Value> {
        &self.registers[index]
    }
}

impl ops::IndexMut<usize> for RegisterFile {
    fn index_mut(&mut self, index: usize) -> &mut Option<Value> {
        &mut self.registers[index]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterSet {
    num_registers: usize,
    set: HashSet<RegisterFile>,
}

impl RegisterSet {
    pub fn new(num_registers: usize) -> Self {
        RegisterSet {
            num_registers,
            set: HashSet::new(),
        }
    }

    pub fn num_registers(&self) -> usize {
        self.num_registers
    }

    pub fn iter(&self) -> RegisterSetIter {
        RegisterSetIter {
            inner: self.set.iter(),
        }
    }

    pub fn push(&mut self, registers: RegisterFile) {
        assert!(self.num_registers() == registers.len());
        self.set.insert(registers);
    }
}

#[derive(Clone, Debug)]
pub struct RegisterSetIter<'a> {
    inner: hash_set::Iter<'a, RegisterFile>,
}

impl<'a> Iterator for RegisterSetIter<'a> {
    type Item = &'a RegisterFile;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
