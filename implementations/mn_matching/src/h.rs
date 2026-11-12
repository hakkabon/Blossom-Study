//! Part II (Sec. 3.1): the auxiliary graph `H`.
//!
//! `H` is built from the search structure *as it was at the beginning of the
//! breakthrough phase*.  Instead of the paper's second ("delayed") union-find
//! we use the blossom stamps: a blossom formed in a phase `< Delta` is a node
//! of `H`; blossoms with stamp `== Delta` are dissolved, and the lcp-values
//! they assigned (logged in `phase_log`) are reverted, i.e. their odd vertices
//! are odd again.

use crate::blossom::{BlossomForest, NONE};
use crate::part1::{Lab, Search};

#[derive(Clone, Copy, Debug)]
pub struct HEdge {
    pub a: usize,
    pub b: usize,
    pub pre: usize,
}

/// Augmenting paths of `H` are in 1-to-1 correspondence with the shortest
/// augmenting paths of `G` (Lemma 9).  `H` is undirected; alternation is given
/// by the induced matching `mate/medge`.
pub struct HGraph {
    pub n: usize,
    pub edges: Vec<HEdge>,
    pub adj: Vec<Vec<(usize, usize)>>,
    pub mate: Vec<usize>,
    pub medge: Vec<usize>,
    /// `H` node whose base is a free vertex of `G`
    pub free: Vec<bool>,
    pub label: Vec<usize>,
    pub top: Vec<usize>,
    pub forest: BlossomForest,
    /// breakthrough phase; saps have length `2*delta - 1`
    pub delta: usize,
}

pub fn construct(mut s: Search<'_>, delta: usize) -> HGraph {
    let g = s.g;
    let n = g.n;
    let d = delta as i64;

    // revert the lcp-values assigned in the breakthrough phase
    for &z in &s.phase_log {
        s.lab[z] = Lab::Odd;
    }

    // nodes: maximal blossoms at the start of the phase
    let mut label = vec![NONE; n];
    let mut top: Vec<usize> = Vec::new();
    for v in 0..n {
        if s.c.dsu.find(v) != v {
            continue;
        }
        let mut stack = vec![s.c.top[v]];
        while let Some(nd) = stack.pop() {
            let f = &s.c.forest;
            if f.is_leaf(nd) || f.stamp(nd) < d {
                let h = top.len();
                top.push(nd);
                let mut st2 = vec![nd];
                while let Some(z) = st2.pop() {
                    if f.is_leaf(z) {
                        label[z] = h;
                    } else {
                        st2.extend(f.cycle(z).iter().map(|&(c, _)| c));
                    }
                }
            } else {
                stack.extend(f.cycle(nd).iter().map(|&(c, _)| c));
            }
        }
    }
    let nh = top.len();

    // edges
    let mut edges = Vec::new();
    let mut adj = vec![Vec::new(); nh];
    let mut mate = vec![NONE; nh];
    let mut medge = vec![NONE; nh];
    for (eid, &(a, b)) in g.edges.iter().enumerate() {
        let (la, lb) = (label[a], label[b]);
        if la == lb {
            continue; // self-loop
        }
        let matched = s.m.medge[a] == eid;
        let keep = match (s.lab[a], s.lab[b]) {
            (Lab::Odd, Lab::Odd) => false,
            (Lab::Even, Lab::Even) => !matched && s.lcp[a] + s.lcp[b] == 2 * d - 2,
            (Lab::Even, Lab::Odd) => odd_even_rule(&s, a, b, matched),
            (Lab::Odd, Lab::Even) => odd_even_rule(&s, b, a, matched),
            // unlabeled-unlabeled: matching edges only
            (Lab::Unlabeled, Lab::Unlabeled) => matched,
            // even-unlabeled edges would be added only for lcp = Delta-2, but all
            // growth of an even phase is already done and odd phases have no such lcp
            _ => false,
        };
        if !keep {
            continue;
        }
        let id = edges.len();
        edges.push(HEdge {
            a: la,
            b: lb,
            pre: eid,
        });
        adj[la].push((lb, id));
        adj[lb].push((la, id));
        if matched {
            mate[la] = lb;
            mate[lb] = la;
            medge[la] = id;
            medge[lb] = id;
        }
    }
    let free: Vec<bool> = (0..nh)
        .map(|h| s.m.mate[s.c.forest.base(top[h])] == NONE)
        .collect();
    HGraph {
        n: nh,
        edges,
        adj,
        mate,
        medge,
        free,
        label,
        top,
        forest: s.c.forest,
        delta,
    }
}

/// even endpoint `u`, odd endpoint `v`
fn odd_even_rule(s: &Search<'_>, u: usize, v: usize, matched: bool) -> bool {
    if matched {
        s.lcp[u] == s.lcp_odd[v] + 1 // matching edges go from odd to the next even layer
    } else {
        s.lcp_odd[v] == s.lcp[u] + 1 // non-matching edges go from even to the next odd layer
    }
}
