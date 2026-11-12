/// An efficient Disjoint-Set Union (DSU) / Union-Find data structure.
///
/// This implementation uses flat vectors for cache locality, path compression
/// to flatten the trees during lookups, and union-by-rank to keep trees balanced.
#[derive(Debug, Clone)]
pub struct UnionFind {
    /// Tracks the parent of each element. If `parent[i] == i`, the element is a root.
    parent: Vec<usize>,
    /// Tracks the tree depth (rank) approximation to keep trees balanced during unions.
    rank: Vec<u32>,
    /// Tracks the total number of disjoint sets inside the structure.
    set_count: usize,
}

impl UnionFind {
    /// Creates a new `UnionFind` instance with `size` isolated elements (0 to size - 1).
    ///
    /// # Time Complexity
    /// $O(N)$ where $N$ is the size.
    pub fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
            set_count: size,
        }
    }

    /// Restore singleton sets, including after a blossom search round.
    pub fn reset(&mut self, size: usize) {
        self.parent.resize(size, 0);
        self.rank.resize(size, 0);
        for (index, parent) in self.parent.iter_mut().enumerate() {
            *parent = index;
        }
        self.rank.fill(0);
        self.set_count = size;
    }

    /// Finds the representative (root) of the set containing `x`.
    ///
    /// This method uses **path compression**, mutably updating the internal parent
    /// pointers to point directly to the root, dramatically speeding up future queries.
    ///
    /// # Panics
    /// Panics if `x` is out of bounds.
    ///
    /// # Time Complexity
    /// Amortized $O(\alpha(N))$, where $\alpha$ is the Inverse Ackermann function.
    pub fn find(&mut self, mut x: usize) -> usize {
        // Assert bounds explicitly to prevent unhelpful panic locations
        assert!(x < self.parent.len(), "Index out of bounds");

        // Iterative path compression to avoid deep call stacks and potential stack overflows.
        // First pass: find the absolute root of the tree.
        let mut root = x;
        while root != self.parent[root] {
            root = self.parent[root];
        }

        // Second pass: update nodes along the path to point directly to the root.
        while x != root {
            let next = self.parent[x];
            self.parent[x] = root;
            x = next;
        }

        root
    }

    /// Unites the sets containing elements `x` and `y`.
    ///
    /// Returns `true` if the sets were successfully merged, and `false` if `x` and `y`
    /// were already part of the same set.
    ///
    /// This method uses **union by rank** to attach the shorter tree under the root of
    /// the taller tree, keeping the overall structure flat.
    ///
    /// # Panics
    /// Panics if `x` or `y` are out of bounds.
    ///
    /// # Time Complexity
    /// Amortized $O(\alpha(N))$ per call.
    pub fn union(&mut self, x: usize, y: usize) -> bool {
        let root_x = self.find(x);
        let root_y = self.find(y);

        // Elements are already in the same set
        if root_x == root_y {
            return false;
        }

        // Union by rank: attach smaller depth tree under larger depth tree
        match self.rank[root_x].cmp(&self.rank[root_y]) {
            std::cmp::Ordering::Less => {
                self.parent[root_x] = root_y;
            }
            std::cmp::Ordering::Greater => {
                self.parent[root_y] = root_x;
            }
            std::cmp::Ordering::Equal => {
                self.parent[root_y] = root_x;
                self.rank[root_x] += 1;
            }
        }

        self.set_count -= 1;
        true
    }

    /// Merge by rank and return the resulting representative.
    ///
    /// Auxiliary blossom arrays indexed by DSU roots must use this returned
    /// value, since the representative can be either input root.
    pub fn union_root(&mut self, x: usize, y: usize) -> usize {
        self.union(x, y);
        self.find(x)
    }

    /// Attach the child's set beneath a *specified* blossom base.
    ///
    /// Union by rank cannot be used here: the alternating-tree algorithms
    /// store their LCA as the representative and depend on that exact ID.
    pub fn union_into(&mut self, child: usize, base: usize) {
        let child = self.find(child);
        let base = self.find(base);
        if child != base {
            self.parent[child] = base;
            self.set_count -= 1;
        }
    }

    /// Alias for ordinary union, used by the retained Korte–Vygen reference.
    pub fn unite(&mut self, x: usize, y: usize) {
        self.union(x, y);
    }

    pub fn is_representant(&mut self, x: usize) -> bool {
        self.find(x) == x
    }

    /// Detach a search vertex when rebuilding the reference forest.
    /// Only valid when the caller knows no other live vertex depends on its
    /// current parent link; this is not an inverse of union.
    pub fn fast_disconnect_element(&mut self, x: usize) {
        if self.parent[x] != x {
            self.parent[x] = x;
            self.rank[x] = 0;
            self.set_count += 1;
        }
    }

    /// Checks if elements `x` and `y` belong to the same set.
    pub fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }

    /// Returns the total number of elements managed by the structure.
    pub fn len(&self) -> usize {
        self.parent.len()
    }

    /// Returns `true` if the structure contains no elements.
    pub fn is_empty(&self) -> bool {
        self.parent.is_empty()
    }

    /// Returns the current number of unique disjoint sets.
    pub fn set_count(&self) -> usize {
        self.set_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_find() {
        let mut dsu = UnionFind::new(10);
        assert_eq!(dsu.set_count(), 10);

        // Merge components
        assert!(dsu.union(0, 1));
        assert!(dsu.union(1, 2));
        assert!(!dsu.union(0, 2)); // Already connected

        assert!(dsu.connected(0, 2));
        assert!(!dsu.connected(0, 3));
        assert_eq!(dsu.set_count(), 8);

        // Build another group
        dsu.union(3, 4);
        dsu.union(4, 5);

        // Merge both groups together
        dsu.union(2, 5);
        assert!(dsu.connected(0, 4));
        assert_eq!(dsu.set_count(), 5);
    }

    #[test]
    fn blossom_merge_keeps_the_chosen_base() {
        let mut dsu = UnionFind::new(6);
        dsu.union_into(1, 3);
        dsu.union_into(2, 3);
        assert_eq!(dsu.find(1), 3);
        assert_eq!(dsu.find(2), 3);
        assert_eq!(dsu.set_count(), 4);
        dsu.reset(6);
        assert_eq!(dsu.find(1), 1);
        assert_eq!(dsu.set_count(), 6);
    }

    #[test]
    fn merge_returns_the_actual_root_for_auxiliary_arrays() {
        let mut dsu = UnionFind::new(4);
        let first = dsu.union_root(0, 1);
        let second = dsu.union_root(2, 3);
        let root = dsu.union_root(first, second);
        for v in 0..4 {
            assert_eq!(dsu.find(v), root);
        }
    }
}
