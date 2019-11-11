use std::collections::hash_map::Entry;
use std::collections::hash_map::IntoIter;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::default::Default;
use std::hash::{BuildHasher, Hash};
use std::iter::Iterator;
use std::iter::{Extend, FromIterator};

#[derive(Debug, Clone, Copy)]
pub struct Data {
    pub parent: usize,
    pub rank: u32,
}

impl Data {
    pub fn new(id: usize) -> Data {
        Data {
            parent: id,
            rank: 0,
        }
    }
}

pub struct SetIter<'a, T>
where
    T: 'a + Eq + Hash,
{
    sets: IntoIter<usize, Vec<&'a T>>,
}

impl<'a, T> Iterator for SetIter<'a, T>
where
    T: 'a + Eq + Hash,
{
    type Item = ::std::vec::IntoIter<&'a T>;

    fn next<'b>(&'b mut self) -> Option<<Self as Iterator>::Item> {
        match self.sets.next() {
            Option::None => None,
            Option::Some((_key, vect)) => Some(vect.into_iter()),
        }
    }
}

impl<'a, T> SetIter<'a, T>
where
    T: 'a + Eq + Hash,
{
    pub fn new(sets: IntoIter<usize, Vec<&'a T>>) -> Self {
        Self { sets }
    }
}

#[derive(Clone, Debug)]
pub struct UnionFind<T, S = RandomState>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    pub ids: HashMap<T, usize, S>,
    pub data_by_id: Vec<Data>,
}

impl<T, S> UnionFind<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    /// Creates a new, empty `UnionFind`.
    pub fn new() -> Self
    where
        S: Default,
    {
        Default::default()
    }

    pub fn with_capacity(capacity: usize) -> Self
    where
        S: Default,
    {
        Self {
            ids: HashMap::with_capacity_and_hasher(capacity, Default::default()),
            data_by_id: Vec::with_capacity(capacity),
        }
    }

    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            ids: HashMap::with_hasher(hash_builder),
            data_by_id: Vec::new(),
        }
    }

    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            ids: HashMap::with_capacity_and_hasher(capacity, hash_builder),
            data_by_id: Vec::with_capacity(capacity),
        }
    }

    pub fn make_set(&mut self, val: T) {
        self.make_or_get_set(val);
    }

    pub fn insert(&mut self, key: T) {
        let val = self.get(&key);
        self.ids.insert(key, val);
    }

    pub fn get(&mut self, key: &T) -> usize {
        *self.ids.get(key).unwrap()
    }

    pub fn union(&mut self, a: T, b: T) {
        let a = self.make_or_get_set(a);
        let b = self.make_or_get_set(b);
        let mut a_root = Self::find_with_path_compression(&mut self.data_by_id, a);
        let mut b_root = Self::find_with_path_compression(&mut self.data_by_id, b);
        if a_root == b_root {
            return;
        }

        if self.data_by_id[a_root].rank < self.data_by_id[b_root].rank {
            let tmp = a_root;
            a_root = b_root;
            b_root = tmp;
        }

        self.data_by_id[b_root].parent = a_root;

        if self.data_by_id[a_root].rank == self.data_by_id[b_root].rank {
            self.data_by_id[a_root].rank += 1;
        }
    }

    pub fn contains(&self, val: &T) -> bool {
        self.ids.contains_key(val)
    }

    pub fn in_union(&mut self, a: &T, b: &T) -> bool {
        let a = match self.ids.get(a) {
            Option::None => return false,
            Option::Some(id) => *id,
        };

        let b = match self.ids.get(b) {
            Option::None => return false,
            Option::Some(id) => *id,
        };

        Self::find_with_path_compression(&mut self.data_by_id, a)
            == Self::find_with_path_compression(&mut self.data_by_id, b)
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.data_by_id.clear()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.data_by_id.reserve(additional);
        self.ids.reserve(additional);
    }

    fn make_or_get_set(&mut self, val: T) -> usize {
        let next_id = self.ids.len();
        //insert but do not override existing one
        match self.ids.entry(val) {
            Entry::Vacant(entry) => {
                entry.insert(next_id);
                //make element its own parent
                self.data_by_id.push(Data::new(next_id));
                next_id
            }
            Entry::Occupied(entry) => *entry.get(),
        }
    }

    fn find_with_path_compression(data_by_id: &mut Vec<Data>, id: usize) -> usize {
        let mut parent = data_by_id[id].parent;
        if parent != id {
            parent = Self::find_with_path_compression(data_by_id, parent);
            data_by_id[id].parent = parent;
        }
        parent
    }

    fn build_sets<'a>(&'a mut self) -> HashMap<usize, Vec<&'a T>> {
        let mut map: HashMap<usize, Vec<&'a T>> = HashMap::new();
        for (ref key, ref val) in self.ids.iter() {
            let root = Self::find_with_path_compression(&mut self.data_by_id, **val);
            map.entry(root).or_insert_with(|| Vec::new()).push(key);
        }
        map
    }
}

impl<T, S> Default for UnionFind<T, S>
where
    T: Eq + Hash,
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self {
            ids: HashMap::default(),
            data_by_id: Vec::default(),
        }
    }
}

impl<T, S> FromIterator<T> for UnionFind<T, S>
where
    T: Hash + Eq,
    S: BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut ds = Self::with_capacity(iter.size_hint().0);
        for val in iter {
            ds.make_set(val);
        }
        ds
    }
}

impl<'a, T, S> FromIterator<&'a T> for UnionFind<T, S>
where
    T: Hash + Eq + Clone,
    S: BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut ds = Self::with_capacity(iter.size_hint().0);
        for val in iter.into_iter().map(|ref val| (*val).clone()) {
            ds.make_set(val)
        }
        ds
    }
}

impl<T, S> Extend<T> for UnionFind<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for val in iter {
            self.make_set(val)
        }
    }
}

impl<'a, T, S> Extend<&'a T> for UnionFind<T, S>
where
    T: Hash + Eq + Copy,
    S: BuildHasher,
{
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for val in iter.map(|&val| val.clone()) {
            self.make_set(val);
        }
    }
}

impl<'a, T, S> IntoIterator for &'a mut UnionFind<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    type Item = ::std::vec::IntoIter<&'a T>;
    type IntoIter = SetIter<'a, T>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        SetIter::new(self.build_sets().into_iter())
    }
}
