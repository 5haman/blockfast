use std::collections::HashMap;
use std::hash::Hash;
use std::result;

/// Tarjan's Union-Find data structure.
pub struct DisjointSet<T: Hash + Eq> {
    pub parent: Vec<usize>,
    pub map: HashMap<T, usize>, // Each T entry is mapped onto a usize tag.
    set_size: usize,
    rank: Vec<usize>,
}

impl<T> DisjointSet<T>
where
    T: Hash + Eq,
{
    pub fn new() -> Self {
        DisjointSet {
            set_size: 0,
            parent: Vec::new(),
            rank: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.set_size
    }

    pub fn make_set(&mut self, x: T) {
        if self.map.contains_key(&x) {
            return;
        }

        let len = &mut self.set_size;
        self.map.insert(x, *len);
        self.parent.push(*len);
        self.rank.push(0);

        *len += 1;
    }

    // Returns Some(num), num is the tag of subset in which x is
    pub fn find(&mut self, x: &T) -> Option<usize> {
        let mut pos: usize;
        match self.map.get(x) {
            Some(p) => {
                pos = *p;
            }
            None => return None,
        }

        while self.parent[pos] != pos {
            self.parent[pos] = self.parent[self.parent[pos]];
            pos = self.parent[pos];
        }

        Some(pos)
    }

    // Union the subsets to which x and y belong.
    pub fn union(&mut self, x: &T, y: &T) -> result::Result<usize, ()> {
        let x_root;
        let y_root;
        let x_rank;
        let y_rank;
        match self.find(&x) {
            Some(x_r) => {
                x_root = x_r;
                x_rank = self.rank[x_root];
            }
            None => {
                return Err(());
            }
        }

        match self.find(&y) {
            Some(y_r) => {
                y_root = y_r;
                y_rank = self.rank[y_root];
            }
            None => {
                return Err(());
            }
        }

        // Implements union-by-rank optimization.
        if x_root == y_root {
            return Ok(x_root);
        }

        if x_rank > y_rank {
            self.parent[y_root] = x_root;
            return Ok(x_root);
        } else {
            self.parent[x_root] = y_root;
            if x_rank == y_rank {
                self.rank[y_root] += 1;
            }
            return Ok(y_root);
        }
    }

    /// Forces all laziness, updating every tag.
    pub fn finalize(&mut self) {
        for _i in 0..self.set_size {
            //DisjointSet::<T>::find_internal(&mut self.parent, i);
        }
    }
}
