use super::*;
// Independent exhaustive recursion: does NOT call any blossom/search routine.
fn brute(n: usize, es: &[Edge], perfect: bool) -> Option<i128> {
    let mut w = vec![vec![None; n]; n];
    for e in es {
        let p = &mut w[e.u][e.v];
        *p = Some(p.map_or(e.weight, |x: i64| x.max(e.weight)));
        w[e.v][e.u] = *p;
    }
    fn go(mask: u64, w: &[Vec<Option<i64>>], perfect: bool) -> Option<i128> {
        if mask == 0 {
            return Some(0);
        }
        let v = mask.trailing_zeros() as usize;
        let rest = mask & !(1 << v);
        let mut best = if perfect { None } else { go(rest, w, perfect) };
        for u in v + 1..w.len() {
            if rest & (1 << u) != 0 {
                if let Some(x) = w[v][u] {
                    if let Some(t) = go(rest & !(1 << u), w, perfect) {
                        let t = t + x as i128;
                        best = Some(best.map_or(t, |b| b.max(t)));
                    }
                }
            }
        }
        best
    }
    go((1u64 << n) - 1, &w, perfect)
}
fn check(n: usize, e: &[Edge], o: &Options) {
    let expected = brute(n, e, false).unwrap();
    let m = maximum_weight_matching_with_options(n, e, o).unwrap();
    assert_eq!(
        m.total_weight, expected,
        "n={} edges={:?}, stats={:?}",
        n, e, m.stats
    );
    for (v, w) in m.pairs() {
        assert_eq!(m.mate[w], Some(v));
        assert!(
            e.iter()
                .any(|e| (e.u == v && e.v == w) || (e.u == w && e.v == v))
        );
    }
    let expected = brute(n, e, true);
    let m = maximum_weight_perfect_matching_with_options(n, e, o);
    match (expected, m) {
        (Some(w), Ok(m)) => {
            assert_eq!(m.total_weight, w, "n={} edges={:?}", n, e);
            assert_eq!(m.cardinality() * 2, n);
        }
        (None, Err(Error::NoPerfectMatching)) => {}
        (a, b) => panic!("expected {:?}, got {:?}; n={}, edges={:?}", a, b, n, e),
    }
}
#[test]
fn tiny() {
    for (n, e) in [
        (0, vec![]),
        (1, vec![]),
        (2, vec![Edge::new(0, 1, 1)]),
        (
            3,
            vec![Edge::new(0, 1, 8), Edge::new(1, 2, 9), Edge::new(2, 0, 10)],
        ),
        (
            4,
            vec![
                Edge::new(0, 1, -8),
                Edge::new(1, 2, 5),
                Edge::new(2, 3, -1),
                Edge::new(0, 3, -10),
            ],
        ),
    ] {
        check(n, &e, &Options::default());
    }
}
#[test]
fn randomized_exhaustive() {
    let mut seed = 0x23916a40c5282bafu64;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for case in 0..6000 {
        let n = (rnd() % 11) as usize;
        let density = rnd() % 101;
        let mut es = vec![];
        for u in 0..n {
            for v in u + 1..n {
                if rnd() % 100 < density {
                    es.push(Edge::new(u, v, (rnd() % 101) as i64 - 40));
                }
            }
        }
        let tau = match case % 5 {
            0 => Some(1),
            1 => Some(2),
            2 => Some(3),
            3 => Some(n.max(1) + 1),
            _ => None,
        };
        if let Err(p) = std::panic::catch_unwind(|| {
            check(
                n,
                &es,
                &Options {
                    tau,
                    check_invariants: true,
                },
            )
        }) {
            eprintln!("FAILING case={} n={} tau={:?} es={:?}", case, n, tau, es);
            std::panic::resume_unwind(p);
        }
    }
}
#[test]
fn edge_cases() {
    check(2, &[Edge::new(0, 1, i64::MIN)], &Options::default());
    check(
        4,
        &[
            Edge::new(0, 1, i64::MAX),
            Edge::new(2, 3, i64::MAX),
            Edge::new(0, 2, i64::MIN),
            Edge::new(1, 3, 0),
        ],
        &Options::default(),
    );
    check(
        3,
        &[Edge::new(0, 1, 1), Edge::new(1, 0, 9), Edge::new(1, 2, 7)],
        &Options::default(),
    );
    assert!(matches!(
        maximum_weight_matching(2, &[Edge::new(0, 2, 1)]),
        Err(Error::InvalidVertex { .. })
    ));
    assert!(matches!(
        maximum_weight_matching(1, &[Edge::new(0, 0, 1)]),
        Err(Error::SelfLoop { .. })
    ));
    assert_eq!(
        maximum_weight_matching_with_options(
            0,
            &[],
            &Options {
                tau: Some(0),
                check_invariants: true
            }
        )
        .unwrap_err(),
        Error::InvalidTau
    );
}
