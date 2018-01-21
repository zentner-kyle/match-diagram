use std::collections::HashSet;
use std::collections::hash_map;
use std::collections::hash_set;
use std::hash;
use std::iter;
use std::ops;

use value::Value;
use weight::Weight;

#[derive(Clone, Debug)]
pub struct RegisterFile {
    registers: Vec<Option<Value>>,
}

impl PartialEq for RegisterFile {
    fn eq(&self, other: &Self) -> bool {
        self.registers.eq(&other.registers)
    }
}

impl Eq for RegisterFile {}

impl hash::Hash for RegisterFile {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        self.registers.hash(hasher);
    }
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

#[derive(Copy, Clone, Debug, PartialEq)]
struct State {
    weight: Weight,
    depth: usize,
}

impl State {
    fn zero() -> Self {
        State {
            weight: Weight(0),
            depth: 0,
        }
    }
}

impl Eq for State {}

#[derive(Clone, Debug)]
pub struct RegisterSet {
    num_registers: usize,
    states: hash_map::HashMap<RegisterFile, State>,
}

impl PartialEq for RegisterSet {
    fn eq(&self, other: &Self) -> bool {
        self.num_registers == other.num_registers
            && self.states.keys().all(|r| other.states.contains_key(r))
            && other.states.keys().all(|r| self.states.contains_key(r))
    }
}

impl Eq for RegisterSet {}

impl RegisterSet {
    pub fn new(num_registers: usize) -> Self {
        RegisterSet {
            num_registers,
            states: hash_map::HashMap::new(),
        }
    }

    pub fn num_registers(&self) -> usize {
        self.num_registers
    }

    pub fn iter(&self) -> RegisterSetIter {
        RegisterSetIter {
            inner: self.states.iter(),
        }
    }

    /**
     * Return whether the state is *new*.
     */
    pub fn push(&mut self, registers: RegisterFile, weight: Weight, depth: usize) -> bool {
        assert!(self.num_registers() == registers.len());
        match self.states.entry(registers) {
            hash_map::Entry::Occupied(mut entry) => {
                if entry.get().depth > depth {
                    entry.get_mut().depth = depth;
                }
                entry.get_mut().weight.0 += weight.0;
                if entry.get().weight.0 == 0 {
                    entry.remove();
                }
                false
            }
            hash_map::Entry::Vacant(entry) => {
                let mut state = State::zero();
                state.weight.0 += weight.0;
                state.depth = depth;
                entry.insert(state);
                true
            }
        }
    }

    pub fn contains(&self, registers: &RegisterFile) -> bool {
        self.states.contains_key(registers)
    }
}

#[derive(Clone, Debug)]
pub struct RegisterSetIter<'a> {
    inner: hash_map::Iter<'a, RegisterFile, State>,
}

impl<'a> Iterator for RegisterSetIter<'a> {
    type Item = (&'a RegisterFile, Weight, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(rs, s)| (rs, s.weight, s.depth))
    }
}
