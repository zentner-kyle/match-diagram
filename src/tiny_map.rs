use std::cmp::PartialEq;
use std::fmt;
use std::mem;
use std::slice;
use std::vec;

pub struct TinyMap<K, V>
where
    K: PartialEq,
{
    data: Vec<(K, V)>,
}

impl<K, V> TinyMap<K, V>
where
    K: PartialEq,
{
    pub fn new() -> Self {
        TinyMap { data: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        TinyMap {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data
            .iter()
            .filter_map(|&(ref k, ref v)| if k.eq(key) { Some(v) } else { None })
            .next()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.data
            .iter_mut()
            .filter_map(|&mut (ref k, ref mut v)| if k.eq(key) { Some(v) } else { None })
            .next()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<(K, V)> {
        let mut key = key;
        let mut value = value;
        if let Some((k, v)) = self.data
            .iter_mut()
            .filter_map(
                |&mut (ref mut k, ref mut v)| if (&*k).eq(&key) { Some((k, v)) } else { None },
            )
            .next()
        {
            mem::swap(k, &mut key);
            mem::swap(v, &mut value);
            return Some((key, value));
        };
        self.data.push((key, value));
        return None;
    }

    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            inner: self.data.iter(),
        }
    }

    pub fn into_iter(self) -> IntoIter<K, V> {
        IntoIter {
            inner: self.data.into_iter(),
        }
    }

    pub fn entry(&mut self, key: K) -> Entry<K, V> {
        let index = self.data.iter().position(|&(ref k, _)| k.eq(&key));
        if let Some(index) = index {
            Entry::Occupied(OccupiedEntry {
                key,
                slot: &mut self.data[index],
            })
        } else {
            Entry::Vacant(VacantEntry {
                key,
                data: &mut self.data,
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, K: 'a, V: 'a> {
    inner: slice::Iter<'a, (K, V)>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|&(ref k, ref v)| (k, v))
    }
}

#[derive(Clone, Debug)]
pub struct IntoIter<K, V> {
    inner: vec::IntoIter<(K, V)>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<K, V> Clone for TinyMap<K, V>
where
    K: Clone + PartialEq,
    V: Clone,
{
    fn clone(&self) -> Self {
        TinyMap {
            data: self.data.clone(),
        }
    }
}

impl<K, V> fmt::Debug for TinyMap<K, V>
where
    K: fmt::Debug + PartialEq,
    V: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_map().entries(self.iter()).finish()
    }
}

pub enum Entry<'a, K: 'a, V: 'a> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    key: K,
    slot: &'a mut (K, V),
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn get(&self) -> &V {
        &self.slot.1
    }

    pub fn get_mut(&mut self) -> &mut V {
        &mut self.slot.1
    }

    pub fn into_mut(self) -> &'a mut V {
        &mut self.slot.1
    }
}

pub struct VacantEntry<'a, K: 'a, V: 'a> {
    key: K,
    data: &'a mut Vec<(K, V)>,
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn insert(self, value: V) -> &'a mut V {
        self.data.push((self.key, value));
        let last = self.data.last_mut().unwrap();
        &mut last.1
    }
}
