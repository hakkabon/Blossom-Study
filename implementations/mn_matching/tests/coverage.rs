use mn_matching::{
    graph::{Graph, Matching},
    h, part1, part3,
};
// Do positive blossoms (and nested ones) really reach H, so lifting is exercised?
#[test]
fn blossoms_reach_h() {
    let mut seed = 777u64;
    let mut rnd = |k: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % k as u64) as usize
    };
    let (mut with_blossom, mut nested, mut iters) = (0, 0, 0);
    for _ in 0..400 {
        let n = 20 + rnd(80);
        let es: Vec<_> = (0..n + rnd(n)).map(|_| (rnd(n), rnd(n))).collect();
        let g = Graph::new(n, &es);
        let mut m = Matching::new(n);
        loop {
            let mut s = part1::Search::new(&g, &m);
            let Some(d) = s.run() else { break };
            let hg = h::construct(s, d);
            iters += 1;
            let nb = hg.top.iter().filter(|&&t| !hg.forest.is_leaf(t)).count();
            if nb > 0 {
                with_blossom += 1;
            }
            if hg.top.iter().any(|&t| {
                !hg.forest.is_leaf(t)
                    && hg
                        .forest
                        .cycle(t)
                        .iter()
                        .any(|&(c, _)| !hg.forest.is_leaf(c))
            }) {
                nested += 1;
            }
            let paths = part3::find_ap_set(&hg);
            let lifted: Vec<_> = paths
                .iter()
                .map(|p| part3::lift_to_g(&g, &m, &hg, p))
                .collect();
            for p in &lifted {
                m.augment(&g, p);
            }
        }
    }
    println!("iterations={iters} with_blossom_in_H={with_blossom} nested={nested}");
    assert!(with_blossom > 0);
}
