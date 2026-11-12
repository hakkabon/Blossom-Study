use gabow_matching::{graph::Graph, graph::Matching, phase1, phase2};
// Does the test-suite really exercise positive blossoms in H and nested lifting?
#[test]
fn blossoms_reach_h() {
    let mut seed = 777u64;
    let mut rnd = |k: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % k as u64) as usize
    };
    let (mut with_blossom, mut nested, mut phases) = (0, 0, 0);
    for _ in 0..400 {
        let n = 20 + rnd(80);
        let es: Vec<_> = (0..n + rnd(n)).map(|_| (rnd(n), rnd(n))).collect();
        let g = Graph::new(n, &es);
        let mut m = Matching::new(n);
        while let Some(h) = phase1::build_help_graph(&g, &m) {
            phases += 1;
            let nb = h.top.iter().filter(|&&t| !h.forest.is_leaf(t)).count();
            if nb > 0 {
                with_blossom += 1;
            }
            if h.top.iter().any(|&t| {
                !h.forest.is_leaf(t) && h.forest.cycle(t).iter().any(|&(c, _)| !h.forest.is_leaf(c))
            }) {
                nested += 1;
            }
            let paths = phase2::find_ap_set(&h);
            let lifted: Vec<_> = paths
                .iter()
                .map(|p| phase2::lift_to_g(&g, &m, &h, p))
                .collect();
            for p in &lifted {
                m.augment(&g, p);
            }
        }
    }
    println!("phases={phases} with_blossom_in_H={with_blossom} nested={nested}");
    assert!(with_blossom > 0);
}
