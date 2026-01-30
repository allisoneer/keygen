use keygen::constraints::Constraints;
use keygen::cost::Config;
use keygen::layout_26::Layout;
use keygen::optimizer::Optimizer;

#[test]
fn test_backward_compatibility() {
    let corpus = "abcdefghijklmnopqrstuvwxyz";
    let cfg = Config::default();
    let opt = Optimizer::new(corpus, cfg);
    let initial = Layout::alphabetical();
    let results = opt.anneal(initial, 1, 1, false);
    assert!(!results.is_empty());
}

#[test]
fn anneal_respects_constraints() {
    let corpus = "you you you you";
    let cfg = Config::default();
    let constraints = Constraints::with_forbid_same_hand_words(&["you"]);
    let opt = Optimizer::new_with_constraints(corpus, cfg, constraints);

    let initial = Layout::alphabetical();
    let results = opt.anneal(initial, 1, 1, false);
    for (layout, _, _) in results {
        let c = Constraints::with_forbid_same_hand_words(&["you"]);
        assert!(
            c.check_layout(&layout),
            "anneal() returned a violating layout"
        );
    }
}

#[test]
fn refine_never_returns_violating_layout() {
    let corpus = "abcdefghijklmnopqrstuvwxyz";
    let cfg = Config::default();
    let constraints = Constraints::with_forbid_same_hand_words(&["you"]);
    let opt = Optimizer::new_with_constraints(corpus, cfg, constraints);
    let initial = Layout::alphabetical(); // violates "you"

    let results = opt.refine(initial, 1, 5, false);
    assert_eq!(results.len(), 1);
    let (layout, _, _) = &results[0];
    let c = Constraints::with_forbid_same_hand_words(&["you"]);
    assert!(
        c.check_layout(layout),
        "refine() returned a violating layout"
    );
}
