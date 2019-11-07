use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub struct UnionFind(Box<[Entry]>);

struct Entry {
    id: AtomicUsize,
    rank: AtomicUsize,
}

impl Clone for Entry {
    fn clone(&self) -> Self {
        Entry::with_rank(
            self.id.load(Ordering::SeqCst),
            self.rank.load(Ordering::SeqCst),
        )
    }
}

impl Default for UnionFind {
    fn default() -> Self {
        UnionFind::new(0)
    }
}

impl Entry {
    fn new(id: usize) -> Self {
        Self::with_rank(id, 0)
    }

    fn with_rank(id: usize, rank: usize) -> Self {
        Entry {
            id: AtomicUsize::new(id),
            rank: AtomicUsize::new(rank),
        }
    }
}

impl UnionFind {
    /// Creates a new asynchronous union-find of `size` elements.
    pub fn new(size: usize) -> Self {
        UnionFind(
            (0..size)
                .map(Entry::new)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }

    /// The number of elements in all the sets.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Is the union-find devoid of elements?
    ///
    /// It is possible to create an empty `UnionFind`, but unlike with
    /// [`UnionFind`](struct.UnionFind.html) it is not possible to add
    /// elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Joins the sets of the two given elements.
    ///
    /// Returns whether anything changed. That is, if the sets were
    /// different, it returns `true`, but if they were already the same
    /// then it returns `false`.
    pub fn union(&self, mut a: usize, mut b: usize) -> bool {
        loop {
            a = self.find(a);
            b = self.find(b);

            if a == b {
                return false;
            }

            let rank_a = self.rank(a);
            let rank_b = self.rank(b);

            if rank_a > rank_b {
                if self.change_parent(b, b, a) {
                    return true;
                }
            } else if rank_b > rank_a {
                if self.change_parent(a, a, b) {
                    return true;
                }
            } else if self.change_parent(a, a, b) {
                self.increment_rank(b);
                return true;
            }
        }
    }

    /// Finds the representative element for the given element’s set.
    pub fn find(&self, mut element: usize) -> usize {
        let mut parent = self.parent(element);

        while element != parent {
            let grandparent = self.parent(parent);
            self.change_parent(element, parent, grandparent);
            element = parent;
            parent = grandparent;
        }

        element
    }

    /// Determines whether two elements are in the same set.
    pub fn equiv(&self, mut a: usize, mut b: usize) -> bool {
        loop {
            a = self.find(a);
            b = self.find(b);

            if a == b {
                return true;
            }
            if self.parent(a) == a {
                return false;
            }
        }
    }

    /// Forces all laziness, so that each element points directly to its
    /// set’s representative.
    pub fn force(&self) {
        for i in 0..self.len() {
            loop {
                let parent = self.parent(i);
                if i == parent {
                    break;
                } else {
                    let root = self.find(parent);
                    if parent == root || self.change_parent(i, parent, root) {
                        break;
                    }
                }
            }
        }
    }

    /// Returns a vector of set representatives.
    pub fn to_vec(&self) -> Vec<usize> {
        self.force();
        self.0
            .iter()
            .map(|entry| entry.id.load(Ordering::SeqCst))
            .collect()
    }

    // HELPERS

    fn rank(&self, element: usize) -> usize {
        self.0[element].rank.load(Ordering::SeqCst)
    }

    fn increment_rank(&self, element: usize) {
        self.0[element].rank.fetch_add(1, Ordering::SeqCst);
    }

    fn parent(&self, element: usize) -> usize {
        self.0[element].id.load(Ordering::SeqCst)
    }

    fn change_parent(&self, element: usize, old_parent: usize, new_parent: usize) -> bool {
        self.0[element]
            .id
            .compare_and_swap(old_parent, new_parent, Ordering::SeqCst)
            == old_parent
    }
}
