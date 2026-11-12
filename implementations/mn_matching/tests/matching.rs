use mn_matching::{Graph, certificate, maximum_matching};

// ---- reference: classic O(n^3) Edmonds blossom (e-maxx style) -------------
fn reference(n: usize, g: &Graph) -> usize {
    const NO: usize = usize::MAX;
    let adj: Vec<Vec<usize>> = (0..n)
        .map(|v| g.adj[v].iter().map(|&(u, _)| u).collect())
        .collect();
    let mut mt = vec![NO; n];
    let mut p = vec![NO; n];
    let mut base = vec![0usize; n];
    let lca = |a0: usize, b0: usize, mt: &Vec<usize>, p: &Vec<usize>, base: &Vec<usize>| {
        let mut used = vec![false; n];
        let mut a = a0;
        loop {
            a = base[a];
            used[a] = true;
            if mt[a] == NO {
                break;
            }
            a = p[mt[a]];
        }
        let mut b = b0;
        loop {
            b = base[b];
            if used[b] {
                return b;
            }
            b = p[mt[b]];
        }
    };
    let find_path =
        |root: usize, mt: &Vec<usize>, p: &mut Vec<usize>, base: &mut Vec<usize>| -> usize {
            let mut used = vec![false; n];
            for i in 0..n {
                p[i] = NO;
                base[i] = i;
            }
            used[root] = true;
            let mut q = std::collections::VecDeque::new();
            q.push_back(root);
            while let Some(v) = q.pop_front() {
                for &to in &adj[v] {
                    if base[v] == base[to] || mt[v] == to {
                        continue;
                    }
                    if to == root || (mt[to] != NO && p[mt[to]] != NO) {
                        let cb = lca(v, to, mt, p, base);
                        let mut blossom = vec![false; n];
                        for (mut x, mut ch) in [(v, to), (to, v)] {
                            while base[x] != cb {
                                blossom[base[x]] = true;
                                blossom[base[mt[x]]] = true;
                                p[x] = ch;
                                ch = mt[x];
                                x = p[mt[x]];
                            }
                        }
                        for i in 0..n {
                            if blossom[base[i]] {
                                base[i] = cb;
                                if !used[i] {
                                    used[i] = true;
                                    q.push_back(i);
                                }
                            }
                        }
                    } else if p[to] == NO {
                        p[to] = v;
                        if mt[to] == NO {
                            return to;
                        }
                        let t = mt[to];
                        used[t] = true;
                        q.push_back(t);
                    }
                }
            }
            NO
        };
    for i in 0..n {
        if mt[i] == NO {
            let mut v = find_path(i, &mt, &mut p, &mut base);
            while v != NO {
                let pv = p[v];
                let ppv = mt[pv];
                mt[v] = pv;
                mt[pv] = v;
                v = ppv;
            }
        }
    }
    mt.iter().filter(|&&x| x != NO).count() / 2
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, k: usize) -> usize {
        (self.next() % k as u64) as usize
    }
}

fn check(g: &Graph, what: &str) {
    let (m, stats) = maximum_matching(g);
    assert!(m.is_valid(g), "{what}: invalid matching");
    for v in 0..g.n {
        if m.mate[v] != usize::MAX {
            let ok = g.adj[v]
                .iter()
                .any(|&(u, e)| u == m.mate[v] && e == m.medge[v]);
            assert!(ok, "{what}: matched pair is not an edge");
        }
    }
    assert_eq!(m.size(), reference(g.n, g), "{what}: not maximum");
    // every path of an iteration is a sap of length 2*Delta-1; lengths strictly grow
    let mut prev = 0;
    for (i, s) in stats.iter().enumerate() {
        let l = 2 * s.breakthrough_phase - 1;
        assert!(
            s.path_lengths.iter().all(|&x| x == l),
            "{what}: iter {i}: lengths {:?} != {l}",
            s.path_lengths
        );
        assert!(
            l > prev,
            "{what}: sap length did not increase ({prev} -> {l})"
        );
        prev = l;
    }
    // independent optimality proof (Sec. 2.3)
    let lab = certificate(g, &m).expect("matching is maximum");
    for &(u, v) in &g.edges {
        assert!(
            lab[u] == 1 || lab[v] == 1 || (lab[u] == lab[v] && lab[u] >= 2),
            "{what}: bad edge label {u}-{v}"
        );
    }
    let mut cnt = std::collections::HashMap::new();
    for &l in &lab {
        *cnt.entry(l).or_insert(0usize) += 1;
    }
    let bound: usize = cnt
        .iter()
        .map(|(&l, &c)| {
            if l == 1 {
                c
            } else if l >= 2 {
                c / 2
            } else {
                0
            }
        })
        .sum();
    assert_eq!(bound, m.size(), "{what}: certificate bound != |M|");
    let bound = 3.0 * (g.n as f64).sqrt() + 3.0;
    assert!(
        (stats.len() as f64) <= bound,
        "{what}: too many phases {}",
        stats.len()
    );
}

#[test]
fn small_structured() {
    let tri = Graph::new(3, &[(0, 1), (1, 2), (2, 0)]);
    check(&tri, "triangle");
    // path of 4, 5-cycle with a tail, two triangles joined, flower
    check(&Graph::new(4, &[(0, 1), (1, 2), (2, 3)]), "path4");
    check(
        &Graph::new(7, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (4, 5), (5, 6)]),
        "c5+tail",
    );
    check(
        &Graph::new(6, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)]),
        "two triangles",
    );
    check(
        &Graph::new(
            8,
            &[
                (0, 1),
                (1, 2),
                (2, 0),
                (2, 3),
                (3, 4),
                (4, 5),
                (5, 3),
                (5, 6),
                (6, 7),
            ],
        ),
        "flowers",
    );
    let k: Vec<(usize, usize)> = (0..9)
        .flat_map(|i| (i + 1..9).map(move |j| (i, j)))
        .collect();
    check(&Graph::new(9, &k), "K9");
    check(&Graph::new(5, &[]), "empty");
}

#[test]
fn random_small() {
    let mut r = Rng(0x9E3779B97F4A7C15);
    for it in 0..4000 {
        let n = 2 + r.below(24);
        let dens = 1 + r.below(8);
        let mut es = vec![];
        for i in 0..n {
            for j in i + 1..n {
                if r.below(20) < dens {
                    es.push((i, j));
                }
            }
        }
        check(&Graph::new(n, &es), &format!("rand#{it} n={n}"));
    }
}

#[test]
fn random_sparse_blossomy() {
    // sparse graphs with many odd cycles; many phases and nested blossoms
    let mut r = Rng(12345);
    for it in 0..300 {
        let n = 30 + r.below(120);
        let m = n + r.below(n);
        let es: Vec<_> = (0..m).map(|_| (r.below(n), r.below(n))).collect();
        check(&Graph::new(n, &es), &format!("sparse#{it} n={n}"));
    }
}

#[test]
fn long_odd_structures() {
    // chain of triangles ("necklace") and a long path: deep recursion & long saps
    let k = 300;
    let mut es = vec![];
    for i in 0..k {
        let b = 3 * i;
        es.extend([(b, b + 1), (b + 1, b + 2), (b + 2, b)]);
        if i + 1 < k {
            es.push((b + 2, b + 3));
        }
    }
    check(&Graph::new(3 * k, &es), "necklace");
    let p: Vec<_> = (0..1999).map(|i| (i, i + 1)).collect();
    check(&Graph::new(2000, &p), "path2000");
}
