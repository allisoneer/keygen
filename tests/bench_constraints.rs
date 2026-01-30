#[test]
fn bench_constraints_overhead_smoke() {
    use keygen::{constraints::Constraints, cost::Config, layout_26::Layout, optimizer::Optimizer};
    use std::time::Instant;

    // Use a small corpus for CI - adjust path as needed
    let corpus = "abcdefghijklmnopqrstuvwxyz ".repeat(100);
    let cfg = Config::default();

    let opt_no = Optimizer::new_with_constraints(&corpus, cfg.clone(), Constraints::default());
    let opt_yes = Optimizer::new_with_constraints(
        &corpus,
        cfg.clone(),
        Constraints::with_forbid_same_hand_words(&["you"]),
    );

    let initial = Layout::alphabetical();

    let t0 = Instant::now();
    let _ = opt_no.anneal(initial.clone(), 1, 1, false);
    let d0 = t0.elapsed();

    let t1 = Instant::now();
    let _ = opt_yes.anneal(initial, 1, 1, false);
    let d1 = t1.elapsed();

    // Sanity check that overhead isn't wildly worse
    // CI environment noise requires relaxed bound
    println!("No constraints: {:?}, With constraints: {:?}", d0, d1);
    assert!(
        d1.as_millis() <= d0.as_millis() * 110 / 100 + 10,
        "Overhead too high: {:?} vs {:?}",
        d1,
        d0
    );
}
