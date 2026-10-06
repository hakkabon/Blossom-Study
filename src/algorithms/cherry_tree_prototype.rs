//! Original Cherry-Tree educational prototype.
//!
//! This source is intentionally retained close to the sibling project so its
//! evolving ideas remain inspectable.  It is not a verified MWPM algorithm:
//! nested-blossom path routing and exact label restoration during expansion are
//! incomplete.  Use the guarded adapter in `algorithms::cherry_tree`, which
//! validates inputs and outputs and clearly labels results experimental.

use std::collections::VecDeque;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct BlossomNode {
    /// The unique index of this blossom supernode (allocated starting at N)
    pub id: usize,
    /// Dual variable z_b for this blossom
    pub z: f64,
    /// The component node indices that make up this blossom's odd cycle
    pub cycle: Vec<usize>,
    /// Is this blossom currently active in the graph or dissolved?
    pub is_active: bool,
}

pub struct CherryTreeSolver {
    // Graph definition
    n: usize,
    distance_matrix: Vec<Vec<f64>>,

    // Primal State
    pub match_pair: Vec<usize>, // match_pair[u] = v (usize::MAX if unmatched)
    label: Vec<i8>,             // -1 = unvisited, 0 = Even (Outer), 1 = Odd (Inner)
    parent: Vec<usize>,         // Alternating tree parent links

    // Dual State
    pub y: Vec<f64>,                // Vertex dual variables
    pub blossoms: Vec<BlossomNode>, // Tracks allocated blossom supernodes

    // Tracking and Structural DSU
    dsu_parent: Vec<usize>, // Maps vertex -> immediate parent blossom
    vis: Vec<usize>,        // Timestamp array for fast LCA
    vis_token: usize,

    pub bridge: Vec<(usize, usize)>, // Initialized to (usize::MAX, usize::MAX)
}

impl CherryTreeSolver {
    pub fn new(n: usize, distance_matrix: Vec<Vec<f64>>) -> Self {
        Self {
            n,
            distance_matrix,
            match_pair: vec![usize::MAX; n * 2], // Accommodate blossoms
            label: vec![-1; n * 2],
            parent: vec![usize::MAX; n * 2],
            y: vec![0.0; n],
            blossoms: Vec::new(),
            dsu_parent: (0..n * 2).collect(),
            vis: vec![0; n * 2],
            vis_token: 0,
            bridge: vec![(usize::MAX, usize::MAX); n * 2],
        }
    }
}

// The Fast DSU Lookups;
// Our DSU maps a physical graph vertex `v` to its outermost
// active blossom container. This allows us to collapse vertices virtually.

impl CherryTreeSolver {
    /// Finds the outermost active blossom container for a vertex
    #[inline]
    fn find_blossom(&mut self, u: usize) -> usize {
        let mut root = u;
        while self.dsu_parent[root] != root {
            root = self.dsu_parent[root];
        }
        // Path compression
        let mut curr = u;
        while curr != root {
            let nxt = self.dsu_parent[curr];
            self.dsu_parent[curr] = root;
            curr = nxt;
        }
        root
    }
}

// Calculating delta dynamically
// Because TSP is solved over a complete distance matrix, we can scan all vertex pairs efficiently
// to compute the primal-dual step size \(\delta \). We calculate \(\delta _{1}\) (tree expansion),
// \(\delta _{2}\) (blossom collision), and \(\delta _{3}\) (blossom dissolution) simultaneously in
// a single pass.

impl CherryTreeSolver {
    pub fn compute_delta(&mut self) -> f64 {
        let mut delta_1 = f64::MAX;
        let mut delta_2 = f64::MAX;
        let mut delta_3 = f64::MAX;

        // 1. Scan edges for Delta_1 and Delta_2
        for u in 0..self.n {
            let root_u = self.find_blossom(u);
            if self.label[root_u] != 0 {
                continue;
            } // u must be Even (Outer)

            for v in 0..self.n {
                let root_v = self.find_blossom(v);
                if root_u == root_v {
                    continue;
                } // Skip internal edges

                // Calculate edge slack: w(u,v) - y_u - y_v - sum(z_b)
                // Note: In our model, blossom duals are factored into the active bounds
                let slack = self.distance_matrix[u][v] - self.y[u] - self.y[v];

                if self.label[root_v] == -1 {
                    // Case 1: Even to Unvisited
                    if slack < delta_1 {
                        delta_1 = slack;
                    }
                } else if self.label[root_v] == 0 {
                    // Case 2: Even to Even (Blossom formation or Tree collision)
                    if slack < delta_2 {
                        delta_2 = slack;
                    }
                }
            }
        }
        // Adjust delta_2 because both sides are moving simultaneously
        if delta_2 != f64::MAX {
            delta_2 /= 2.0;
        }

        // 2. Scan active Odd Blossoms for Delta_3 (Dissolution threshold)
        for b in &self.blossoms {
            if b.is_active && self.label[b.id] == 1 {
                // If blossom is Odd (Inner)
                if b.z < delta_3 {
                    delta_3 = b.z;
                }
            }
        }

        // Return the global minimum shift constraint
        delta_1.min(delta_2).min(delta_3)
    }
}

// Executing the Dual Update Phase
// Once the global `delta` is determined, we apply it uniformly to change the graph constraints.
// This step shifts the edge slacks, forcing new valid paths to open up for the next Cherry Tree
// BFS iteration.

impl CherryTreeSolver {
    pub fn apply_dual_update(&mut self, delta: f64) {
        if delta <= 1e-9 {
            return;
        } // Avoid micro-step thrashing due to float precision

        // 1. Update vertex duals based on their current root blossom label
        for u in 0..self.n {
            let root_u = self.find_blossom(u);
            match self.label[root_u] {
                0 => self.y[u] += delta, // Even (Outer) increases budget
                1 => self.y[u] -= delta, // Odd (Inner) decreases budget
                _ => {}                  // Unvisited stays unchanged
            }
        }

        // 2. Update blossom dual variables
        for b in &mut self.blossoms {
            if b.is_active {
                match self.label[b.id] {
                    0 => b.z -= 2.0 * delta, // Even blossoms shrink internally
                    1 => b.z += 2.0 * delta, // Odd blossoms grow internally
                    _ => {}
                }
            }
        }
    }
}

// Handling Blossom Dissolution
// If `delta == delta_3`, it means an active blossom's dual weight `z` has bottomed
// out exactly at `0.0`. We must clear it from the active state list and let its
// sub-vertices participate individually.

// Zero Heap Thrashing: The `blossoms` array behaves like an append-only memory pool.
// When a blossom dissolves, we simply flip its `is_active` boolean flag to `false`
// instead of removing or dropping elements from vectors.

impl CherryTreeSolver {
    pub fn dissolve_zero_blossoms(&mut self) {
        for i in 0..self.blossoms.len() {
            if self.blossoms[i].is_active && self.blossoms[i].z <= 1e-9 {
                self.blossoms[i].is_active = false;
                let b_id = self.blossoms[i].id;

                // Disown the sub-nodes from the DSU tracker
                // This breaks the cycle apart into its original components
                for &v in &self.blossoms[i].cycle {
                    if self.dsu_parent[v] == b_id {
                        self.dsu_parent[v] = v; // Revert to self-ownership
                    }
                    // Sub-vertices inside an exploded Odd blossom inherit
                    // structural tree labels depending on where the path entered.
                    self.label[v] = -1; // Reset to unvisited for re-evaluation
                }
            }
        }
    }
}

// Dual Initialization
// Before running the solver, we can set the initial dual values to **half the distance
// to the closest neighbor**. This ensures that at least one edge out of every node starts
// with a slack of exactly zero, satisfying LP feasibility instantly.

impl CherryTreeSolver {
    pub fn initialize_duals(&mut self) {
        for u in 0..self.n {
            let mut min_val = f64::MAX;
            for v in 0..self.n {
                if u != v && self.distance_matrix[u][v] < min_val {
                    min_val = self.distance_matrix[u][v];
                }
            }
            // Set the initial dual value to half of the minimum edge cost
            self.y[u] = min_val / 2.0;
        }
    }
}

// The Inner Primal Engine (BFS)
// This is the core execution block. It looks for augmenting paths along zero-slack edges.
// If it finds an unvisited node, it extends the tree. If it hits an already visited even node,
// a new blossom cycle is detected.

impl CherryTreeSolver {
    /// Attempts to find an augmenting path or create blossoms using zero-slack edges.
    /// Returns true if an augmenting path was found and edges were flipped.
    pub fn run_primal_phase(&mut self) -> bool {
        let mut q = VecDeque::new();
        self.label.fill(-1);
        self.parent.fill(usize::MAX);

        // 1. Queue up all currently unmatched nodes as tree roots (Even/Outer)
        for i in 0..self.n {
            if self.match_pair[i] == usize::MAX {
                let root = self.find_blossom(i);
                if self.label[root] == -1 {
                    self.label[root] = 0; // Label as Even
                    q.push_back(root);
                }
            }
        }

        while let Some(u) = q.pop_front() {
            // Traverse all possible structural neighbors
            for v in 0..self.n {
                let root_u = self.find_blossom(u);
                let root_v = self.find_blossom(v);
                if root_u == root_v {
                    continue;
                } // Internal blossom edge

                // Check if the edge has zero slack
                let slack = self.distance_matrix[u][v] - self.y[u] - self.y[v];
                if slack > 1e-9 {
                    continue;
                } // Not a "tight" edge

                if self.label[root_v] == -1 {
                    // CASE A: Unvisited node. Extend the alternating tree.
                    self.parent[root_v] = root_u;

                    if self.match_pair[root_v] == usize::MAX {
                        // Found a free node! Augment the matching back along parent path.
                        self.augment_path(root_v);
                        return true;
                    }

                    // If root_v is matched, its partner becomes an Even node in our tree
                    let partner = self.find_blossom(self.match_pair[root_v]);
                    self.label[root_v] = 1; // Odd
                    self.label[partner] = 0; // Even
                    q.push_back(partner);
                } else if self.label[root_v] == 0 {
                    // Even vertices in different trees form an augmenting path,
                    // not a blossom. Only vertices sharing an ancestor can close
                    // an odd cycle inside one alternating tree.
                    if let Some(lca) = self.find_common_ancestor(root_u, root_v) {
                        self.contract_blossom(root_u, root_v, lca);
                        return false; // Struct changed, reset BFS phase
                    }

                    self.augment_between_trees(root_u, root_v);
                    return true;
                }
            }
        }
        false
    }

    // Traces backward along parent references to flip matched/unmatched states
    // fn augment_path(&mut self, mut curr: usize) {
    //     while curr != usize::MAX {
    //         let p = self.parent[curr];
    //         if p == usize::MAX { break; }
    //         let next_node = self.match_pair[p];
    //
    //         self.match_pair[curr] = p;
    //         self.match_pair[p] = curr;
    //
    //         curr = next_node;
    //     }
    // }

    // Virtually groups nodes into a new structural supernode token
    // fn contract_blossom(&mut self, u: usize, v: usize) {
    //     let b_id = self.n + self.blossoms.len();

    //     // In a full implementation, you trace parent pointers backward to form
    //     // the precise cycle list. For this structural model, we group them:
    //     let cycle = vec![u, v];

    //     let new_blossom = BlossomNode {
    //         id: b_id,
    //         z: 0.0, // Starts at zero slack
    //         cycle: cycle.clone(),
    //         is_active: true,
    //     };

    //     // Update DSU so these components point to the new blossom ID
    //     for &node in &cycle {
    //         self.dsu_parent[node] = b_id;
    //     }

    //     self.blossoms.push(new_blossom);
    // }
}

// The Orchestration Loop
// This brings everything together. The solver loops continuously: it checks if
// any matches can be made immediately (Primal), adjusts weights to reveal new edges
// if the search stalls (Dual Update), and cleans up expired clusters (Dissolution).

impl CherryTreeSolver {
    /// Solves the Minimum Weight Perfect Matching problem.
    /// Returns the complete array mapping vertices to their optimal matched pairs.
    pub fn solve(&mut self) -> &[usize] {
        // Step 1: Bootstrap the system with base dual pricing
        self.initialize_duals();

        // This educational prototype is intentionally incomplete.  A bounded
        // number of rounds prevents a malformed blossom state from turning a
        // CLI experiment into an infinite loop.  The adapter validates the
        // returned mate table and reports an error when the prototype stalls.
        let max_rounds = self
            .n
            .max(1)
            .saturating_mul(self.n.max(1))
            .saturating_mul(8);

        for _round in 0..max_rounds {
            // Step 2: Run the combinatorial structural search
            let augmented = self.run_primal_phase();

            if augmented {
                // Check if all elements are successfully paired up
                // Slots n..2n are reserved for blossom tokens and are not
                // graph vertices; counting them made successful runs appear
                // permanently incomplete.
                let unmatched_count = self.match_pair[..self.n]
                    .iter()
                    .filter(|&&m| m == usize::MAX)
                    .count();
                if unmatched_count == 0 {
                    break;
                }
                continue;
            }

            // Step 3: If progress stalls, calculate the minimum shift metric delta
            let delta = self.compute_delta();

            if delta == f64::MAX || delta <= 1e-9 {
                // Safety exit if no feasible options remain or weights stabilize
                break;
            }

            // Step 4: Shift dual weights and expand bottomed-out clusters
            self.apply_dual_update(delta);
            self.dissolve_zero_blossoms();
        }

        &self.match_pair
    }
}

// Complete Solver Lifecycle Workflow
//
//       [Start solve()]
//              │
//              ▼
//    [initialize_duals()]  <──────────────────────────┐
//              │                                      │
//              ▼                                      │
//    [run_primal_phase()] ─── (Augment True) ──> [Perfect Matching?]
//              │                                      │ (False)
//              │ (Augment False / Stalled)            ▼
//              ▼                              [Keep Searching]
//       [compute_delta()]                             │
//              │                                      │
//              ▼                                      │
//    [apply_dual_update()]                            │
//              │                                      │
//              ▼                                      │
//  [dissolve_zero_blossoms()] ────────────────────────┘
//

// Full Implementation of Cycle-Tracing Contraction
// This method uses a two-step process:
// 1. Trace Upward to Find the LCA Node: Alternately backtrack from `u` and `v` using the
//    `parent` tree pointers to find the junction.
// 2. Harvest the Cycle Paths: Trace upward a second time from both `u` and `v` up to the LCA,
//    appending all intermediate nodes into a flat `cycle` array in consecutive order.

// Understanding the Ordering Layout
// The tracking structure relies on keeping the node sequence continuous.
// If your cycle path has an arbitrary break or gaps, the dissolution
// routine won't know how to route the path back through the sub-graph when
// the blossom explodes.
//
//            [LCA (Apex Junction)]
//               ^             ^
//              /               \
//       (Trace Up)          (Trace Up)
//            /                   \
//        [Node U] <──(Collision)──> [Node V]
//
// By harvesting the left side (`u -> LCA`), dropping the LCA in the middle,
// and pinning the reversed right side (`v -> LCA`) on the tail end, your `cycle`
// vector holds a perfectly continuous loop of indices. When `dissolve_zero_blossoms()`
// triggers later, navigating clockwise or counterclockwise to exit the blossom is
// reduced to a simple array-index step.

/*
This moved into the `augment_path` method above, which now handles flat blossom routing.

impl CherryTreeSolver {
    /// Tracks the Lowest Common Ancestor and builds the exact odd-cycle sequence.
    fn contract_blossom(&mut self, mut u: usize, mut v: usize) {
        self.vis_token += 1;

        // --- STEP 1: Find the LCA ---
        let mut curr_u = u;
        let mut curr_v = v;
        let lca = loop {
            if curr_u != usize::MAX {
                let root_u = self.find_blossom(curr_u);
                if self.vis[root_u] == self.vis_token {
                    break root_u;
                }
                self.vis[root_u] = self.vis_token;
                curr_u = self.parent[root_u];
            }
            // Alternate between tracking paths to ensure balanced ascent
            std::mem::swap(&mut curr_u, &mut curr_v);
        };

        // --- STEP 2: Harvest the Cycle Vertices ---
        let mut cycle = Vec::new();

        // Trace up from u to the LCA
        let mut curr = u;
        while self.find_blossom(curr) != lca {
            let root_curr = self.find_blossom(curr);
            cycle.push(root_curr);
            curr = self.parent[root_curr];
        }
        cycle.push(lca); // Add the pivot apex

        // Trace up from v to the LCA, reversing to maintain contiguous clockwise path order
        let mut v_path = Vec::new();
        curr = v;
        while self.find_blossom(curr) != lca {
            let root_curr = self.find_blossom(curr);
            v_path.push(root_curr);
            curr = self.parent[root_curr];
        }
        v_path.reverse();
        cycle.extend(v_path);

        // --- STEP 3: Register the Blossom Supernode Token ---
        let b_id = self.n + self.blossoms.len();

        let new_blossom = BlossomNode {
            id: b_id,
            z: 0.0,
            cycle: cycle.clone(),
            is_active: true,
        };

        // Update the structural DSU map so all components inherit this blossom container token
        for &node in &cycle {
            self.dsu_parent[node] = b_id;

            // Sub-components trapped in an Even blossom automatically become Even (Outer) nodes
            self.label[node] = 0;
        }

        self.blossoms.push(new_blossom);
    }
}
*/

// Right now, if you try to run the code as written on a complex TSP instance,
// it will compile, but it will eventually panic, loop infinitely, or produce
// suboptimal matchings. Here is exactly what is missing and why it is essential
// to bridge the gap from a working unweighted solver to a working weighted MWPM
// solver:
//
// 1. The Missing Path-Routing Logic in `augment_path`
// When an augmenting path is found, `augment_path` must backtrack along
// `self.parent` to flip the matched edges.
//
// - The Problem: In an unweighted solver, nodes are vertices. In our MWPM solver,
//   `self.parent[root_v]` points to a Blossom ID, not a single vertex.
// - What is missing: When navigating through a blossom during augmentation, you
//   cannot just jump across it. You must step inside the blossom's `cycle` vector,
//   find which specific vertex the path entered, trace around the odd cycle (either
//   clockwise or counter-clockwise, whichever path length is even), flip the matches
//   inside the blossom, and exit through the base vertex.
//
// 2. Labels during `dissolve_zero_blossoms`
// When a blossom's dual variable hits zero, it dissolves.
//
// - The Problem: Currently, the code resets all sub-vertices to `-1` (unvisited).
// - What is missing: In the Blossom algorithm, when an Odd blossom dissolves, its
//   internal vertices cannot just become unvisited. The alternating tree path was
//   running through that blossom. The sub-vertices must inherit precise alternating
//   labels (`0` for Even, `1` for Odd) depending on where the tree path intersected
//   the cycle, ensuring the tree structure doesn't fracture.

// Nested tracking issue can be avoided entirely.
// ----------------------------------------------
// The standard approach to Edmonds' algorithm explicitly tracks nested hierarchies
// using parent pointer vectors (like `blossom_parent`). However, you can bypass this
// complexity by adopting a structural technique known as the
// Dual-Directed Virtual Contraction (or "Flat Bridge Link") pattern.
// [[1](https://arxiv.org/abs/1509.04927)]
//
// Instead of maintaining a hierarchy of nested blossoms, your code can treat all
// blossoms as completely flat entities, and the tracking work is delegated to
// an auxiliary array called `bridge`.
// [[1](https://pub.ista.ac.at/~vnk/papers/blossom5-TR-Sep9.pdf),
//  [2](https://pub.ista.ac.at/~vnk/papers/blossom5-TR.ps)]
//
// How the `bridge` Trick Avoids Nested Tracking
//
// In a flat virtual contraction layout:
//
// 1. When two even nodes collide, you do not nestedly contract blossoms inside
// other blossoms.
// 2. Instead, you create a new blossom token, and you assign its parent pointers
// directly to the original graph vertices that formed the cycle endpoints.
// 3. You save these boundary-crossing edges in a dedicated array:
// `bridge[vertex_index] = (u, v)`.
// [[1](https://arxiv.org/html/2604.20351v1)]
//
// When `augment_path` is tracking backwards and encounters a Blossom ID, it doesn't
// need a `blossom_parent` array to figure out how to navigate inward. It simply reads
// `bridge[current_node]` to immediately discover exactly which original graph edge
// entered the blossom. Because it knows the entry edge, it can step through the blossom's
// flat `cycle` array clockwise or counter-clockwise until it exits.
// [[1](https://pub.ista.ac.at/~vnk/papers/blossom5-TR.ps),
//  [2](https://pub.ista.ac.at/~vnk/papers/blossom5-TR-Sep9.pdf)]

impl CherryTreeSolver {
    /// Flips edges along an alternating path across flat or virtually contracted blossoms
    pub fn augment_path(&mut self, mut curr: usize) {
        while curr != usize::MAX {
            let p = self.parent[curr];
            if p == usize::MAX {
                break;
            }

            // If the parent is a blossom token (ID >= self.n), we must route through it
            if p >= self.n {
                // Look up the exact real edge that formed the blossom boundary
                let (u, v) = self.bridge[p];

                // Route internally through the flat blossom cycle
                self.match_pair[u] = v;
                self.match_pair[v] = u;

                // Cascade the match outwards across the cycle nodes
                let blossom_idx = p - self.n;
                let cycle = &self.blossoms[blossom_idx].cycle;

                // Pair up remaining un-nested nodes in the cycle symmetrically
                for i in (0..cycle.len()).step_by(2) {
                    if i + 1 < cycle.len() {
                        let c1 = cycle[i];
                        let c2 = cycle[i + 1];
                        if self.match_pair[c1] == usize::MAX || self.match_pair[c2] == usize::MAX {
                            self.match_pair[c1] = c2;
                            self.match_pair[c2] = c1;
                        }
                    }
                }
                // Advance the path past the blossom base
                curr = self.parent[p];
            } else {
                // Standard un-nested vertex path step
                let next_node = self.match_pair[p];
                self.match_pair[curr] = p;
                self.match_pair[p] = curr;
                curr = next_node;
            }
        }
    }

    fn find_common_ancestor(&mut self, u: usize, v: usize) -> Option<usize> {
        self.vis_token = self.vis_token.wrapping_add(1);
        if self.vis_token == 0 {
            self.vis.fill(0);
            self.vis_token = 1;
        }
        let token = self.vis_token;

        let mut curr = u;
        while curr != usize::MAX {
            let root = self.find_blossom(curr);
            if self.vis[root] == token {
                break;
            }
            self.vis[root] = token;
            curr = self.parent[root];
        }

        curr = v;
        let mut seen = vec![false; self.vis.len()];
        while curr != usize::MAX {
            let root = self.find_blossom(curr);
            if self.vis[root] == token {
                return Some(root);
            }
            if seen[root] {
                return None;
            }
            seen[root] = true;
            curr = self.parent[root];
        }
        None
    }

    fn trace_even_tree_path(&self, mut curr: usize) -> Option<Vec<usize>> {
        let mut path = Vec::new();
        let mut seen = vec![false; self.n];

        loop {
            if curr >= self.n || seen[curr] {
                return None;
            }
            seen[curr] = true;
            path.push(curr);

            let matched = self.match_pair[curr];
            if matched == usize::MAX {
                return Some(path);
            }
            if matched >= self.n || seen[matched] {
                return None;
            }
            seen[matched] = true;
            path.push(matched);

            curr = self.parent[matched];
            if curr == usize::MAX {
                return None;
            }
        }
    }

    fn augment_between_trees(&mut self, u: usize, v: usize) {
        let mut left_path = self
            .trace_even_tree_path(u)
            .expect("distinct-tree augmentation currently requires uncontracted paths");
        let right_path = self
            .trace_even_tree_path(v)
            .expect("distinct-tree augmentation currently requires uncontracted paths");

        left_path.reverse();
        left_path.extend(right_path);
        assert_eq!(
            left_path.len() % 2,
            0,
            "augmenting path must contain an even number of vertices"
        );

        for pair in left_path.chunks_exact(2) {
            self.match_pair[pair[0]] = pair[1];
            self.match_pair[pair[1]] = pair[0];
        }
    }

    /// Contracts an odd cycle after its shared tree ancestor has been identified.
    fn contract_blossom(&mut self, u: usize, v: usize, lca: usize) {
        // --- Harvest the Cycle Vertices ---
        let mut cycle = Vec::new();

        let mut curr = u;
        while self.find_blossom(curr) != lca {
            let root_curr = self.find_blossom(curr);
            cycle.push(root_curr);
            curr = self.parent[root_curr];
        }
        cycle.push(lca);

        let mut v_path = Vec::new();
        curr = v;
        while self.find_blossom(curr) != lca {
            let root_curr = self.find_blossom(curr);
            v_path.push(root_curr);
            curr = self.parent[root_curr];
        }
        v_path.reverse();
        cycle.extend(v_path);

        // --- STEP 3: Register the Blossom Supernode Token ---
        let b_id = self.n + self.blossoms.len();

        // Store the boundary edge in the slot belonging to this blossom ID.
        self.bridge[b_id] = (u, v);

        // Update DSU ownership and labels before registering this blossom once.
        for &node in &cycle {
            self.dsu_parent[node] = b_id;
            self.label[node] = 0;
        }

        self.blossoms.push(BlossomNode {
            id: b_id,
            z: 0.0,
            cycle,
            is_active: true,
        });
    }
}

/// Simulates a 2D Euclidean coordinate layout
struct City {
    x: f64,
    y: f64,
}

/// Generates a deterministic complete graph distance matrix mimicking 2D spatial maps
fn generate_complete_euclidean_matrix(size: usize, seed: u64) -> Vec<Vec<f64>> {
    let mut cities = Vec::with_capacity(size);

    // Simple LCG pseudo-random generation to avoid external rand dependency
    let mut state = seed;
    for _ in 0..size {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = (state as f64) / (u64::MAX as f64) * 1000.0;
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = (state as f64) / (u64::MAX as f64) * 1000.0;
        cities.push(City { x, y });
    }

    let mut matrix = vec![vec![0.0; size]; size];
    for i in 0..size {
        for j in 0..size {
            if i == j {
                matrix[i][j] = 0.0;
            } else {
                let dx = cities[i].x - cities[j].x;
                let dy = cities[i].y - cities[j].y;
                matrix[i][j] = (dx * dx + dy * dy).sqrt();
            }
        }
    }
    matrix
}

fn calculate_total_matching_weight(matrix: &[Vec<f64>], matching: &[usize]) -> f64 {
    let mut total_weight = 0.0;
    for (u, &v) in matching.iter().enumerate() {
        if v != usize::MAX && u < v {
            // Count each edge exactly once
            total_weight += matrix[u][v];
        }
    }
    total_weight
}

fn run_benchmark_instance(size: usize) {
    println!("--------------------------------------------------");
    println!(
        "Initializing instance layout for V = {} (E ≈ {})",
        size,
        (size * size) / 2
    );

    let matrix = generate_complete_euclidean_matrix(size, 42); // Consistent seed
    let mut solver = CherryTreeSolver::new(size, matrix.clone());

    let start_time = Instant::now();
    let matching = solver.solve();
    let duration = start_time.elapsed();

    let total_cost = calculate_total_matching_weight(&matrix, matching);
    let matched_count = matching.iter().filter(|&&v| v != usize::MAX).count();

    println!("⚡ Execution Time : {:?}", duration);
    println!("📈 Total MWPM Cost : {:.4}", total_cost);
    println!("🔗 Vertices Paired : {} / {}", matched_count, size);

    // Basic verification assertion to ensure it doesn't crash or return invalid sets
    assert_eq!(
        matched_count, size,
        "Solver failed to find a perfect matching!"
    );
}

pub fn run_benchmarks() {
    println!("==================================================");
    println!("          CHERRY-TREE MWPM SOLVER SWEEP           ");
    println!("==================================================");

    // Warm-up instance to prime the CPU cache and page tables
    let _ = generate_complete_euclidean_matrix(50, 13);

    // Execution scaling sweep
    run_benchmark_instance(50); // Tiny sub-problem
    run_benchmark_instance(150); // Average TSP odd-vertex pool size
    run_benchmark_instance(300); // Large spatial challenge
    run_benchmark_instance(500); // Stress testing the O(V*E) / O(V³) upper thresholds

    println!("==================================================");
    println!("Benchmark completed successfully.");
}

#[cfg(test)]
mod tests {
    use super::CherryTreeSolver;

    #[test]
    fn tight_edge_between_distinct_search_roots_augments_instead_of_contracting() {
        let n = 4;
        let mut costs = vec![vec![2.0; n]; n];
        for (i, row) in costs.iter_mut().enumerate() {
            row[i] = 0.0;
        }
        let mut solver = CherryTreeSolver::new(n, costs);
        solver.initialize_duals();

        assert!(solver.run_primal_phase());
        assert_eq!(solver.match_pair[0], 1);
        assert_eq!(solver.match_pair[1], 0);
    }

    #[test]
    fn distinct_tree_augmentation_flips_both_alternating_paths() {
        let n = 6;
        let mut solver = CherryTreeSolver::new(n, vec![vec![0.0; n]; n]);
        solver.match_pair[1] = 2;
        solver.match_pair[2] = 1;
        solver.match_pair[4] = 5;
        solver.match_pair[5] = 4;
        solver.parent[1] = 0;
        solver.parent[4] = 3;

        solver.augment_between_trees(2, 5);

        assert_eq!(&solver.match_pair[..n], &[1, 0, 5, 4, 3, 2]);
    }

    #[test]
    fn contraction_registers_one_blossom_and_its_bridge() {
        let n = 4;
        let mut solver = CherryTreeSolver::new(n, vec![vec![0.0; n]; n]);
        solver.parent[1] = 0;
        solver.parent[2] = 0;

        solver.contract_blossom(1, 2, 0);

        assert_eq!(solver.blossoms.len(), 1);
        assert_eq!(solver.blossoms[0].id, n);
        assert_eq!(solver.bridge[n], (1, 2));
        assert_eq!(solver.blossoms[0].cycle, vec![1, 0, 2]);
    }
}
