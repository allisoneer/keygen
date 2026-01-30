use rand::random;
/// Optimizer for 26-key layouts using simulated annealing and local refinement
use std::collections::HashMap;

use crate::annealing;
use crate::constraints::Constraints;
use crate::coordinator::WorkerEvent;
use crate::corpus;
use crate::cost::{self, Config};
use crate::layout_26::Layout;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
struct LayoutEntry {
    layout: Layout,
    penalty: f64,
}

pub struct Optimizer {
    quartads: HashMap<[u8; 4], usize>,
    config: Config,
    monograms: [usize; 26],
    letters_len: usize,
    constraints: Constraints,
}

impl Optimizer {
    /// Create optimizer with explicit constraints.
    ///
    /// Use this to add hard constraints that categorically reject layouts.
    /// Constraints are checked before expensive scoring, improving performance.
    pub fn new_with_constraints(corpus: &str, config: Config, constraints: Constraints) -> Self {
        let normalized = corpus::normalize(corpus);
        let quartads = corpus::quartads(&normalized);
        let monograms = corpus::monograms(&normalized);
        let letters_len = corpus::letter_count(&normalized);
        Optimizer {
            quartads,
            config,
            monograms,
            letters_len,
            constraints,
        }
    }

    // Preserve existing API
    pub fn new(corpus: &str, config: Config) -> Self {
        Self::new_with_constraints(corpus, config, Constraints::default())
    }

    #[inline]
    fn is_layout_valid(&self, layout: &Layout) -> bool {
        self.constraints.check_layout(layout)
    }

    // Accessor methods for coordinator
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_quartads(&self) -> &HashMap<[u8; 4], usize> {
        &self.quartads
    }

    pub fn get_monograms(&self) -> &[usize; 26] {
        &self.monograms
    }

    pub fn get_letters_len(&self) -> usize {
        self.letters_len
    }

    /// Run simulated annealing optimization
    pub fn anneal(
        &self,
        initial_layout: Layout,
        num_swaps: usize,
        top_n: usize,
        debug: bool,
    ) -> Vec<(Layout, f64, Vec<cost::PenaltyComponent>)> {
        let mut best_layouts: Vec<LayoutEntry> = Vec::new();
        let mut accepted_layout = initial_layout;
        let (initial_penalty, _) = cost::score_all(
            &self.quartads,
            &self.monograms,
            self.letters_len,
            &accepted_layout,
            &self.config,
        );
        let mut accepted_penalty = initial_penalty / self.letters_len as f64;

        if debug {
            println!("Initial penalty: {}", accepted_penalty);
        }

        let mut rej_total = 0usize;
        let mut chk_total = 0usize;
        let mut rej_window = 0usize;
        let mut chk_window = 0usize;

        for i in annealing::get_simulation_range() {
            // Create a new layout by shuffling
            let mut curr_layout = accepted_layout.clone();
            let swaps = random::<usize>() % num_swaps + 1;
            curr_layout.shuffle(swaps);

            // Hard constraints: pre-scoring rejection
            chk_total += 1;
            chk_window += 1;
            if !self.is_layout_valid(&curr_layout) {
                rej_total += 1;
                rej_window += 1;
                if debug && i % 1000 == 0 && i > 0 {
                    let pct = if chk_window > 0 {
                        100.0 * (rej_window as f64) / (chk_window as f64)
                    } else { 0.0 };
                    println!("[constraints] last 1000: {} rejected ({:.1}%), total: {} / {} ({:.1}%)",
                        rej_window, pct,
                        rej_total, chk_total,
                        100.0 * (rej_total as f64) / (chk_total as f64)
                    );
                    rej_window = 0;
                    chk_window = 0;
                }
                continue;
            }

            // Calculate penalty
            let (total_penalty, _) = cost::score_all(
                &self.quartads,
                &self.monograms,
                self.letters_len,
                &curr_layout,
                &self.config,
            );
            let scaled_penalty = total_penalty / self.letters_len as f64;

            // Accept or reject based on simulated annealing
            if annealing::accept_transition(scaled_penalty - accepted_penalty, i) {
                if debug {
                    println!("Iteration {} accepted with penalty {}", i, scaled_penalty);
                }

                accepted_layout = curr_layout.clone();
                accepted_penalty = scaled_penalty;

                // Add to best layouts
                let entry = LayoutEntry {
                    layout: curr_layout,
                    penalty: scaled_penalty,
                };

                // Insert sorted
                let pos = best_layouts
                    .iter()
                    .position(|e| e.penalty > entry.penalty)
                    .unwrap_or(best_layouts.len());
                best_layouts.insert(pos, entry);

                // Keep only top N
                if best_layouts.len() > top_n {
                    best_layouts.pop();
                }
            }
        }

        // Return detailed results for top layouts
        best_layouts
            .into_iter()
            .map(|entry| {
                let (total, components) = cost::score_all(
                    &self.quartads,
                    &self.monograms,
                    self.letters_len,
                    &entry.layout,
                    &self.config,
                );
                (entry.layout, total / self.letters_len as f64, components)
            })
            .collect()
    }

    /// Run simulated annealing with streaming events
    pub fn anneal_streaming<F>(
        &self,
        mut accepted_layout: Layout,
        num_swaps: usize,
        seed: u64,
        stop_flag: Arc<AtomicBool>,
        mut on_event: F,
    ) where
        F: FnMut(WorkerEvent),
    {
        let (initial_penalty, _) = cost::score_all(
            &self.quartads,
            &self.monograms,
            self.letters_len,
            &accepted_layout,
            &self.config,
        );
        let mut accepted_penalty = initial_penalty / self.letters_len as f64;
        let mut best_penalty = accepted_penalty;
        let mut best_layout = accepted_layout.clone();
        let mut accepted = 0u64;

        let mut i = 1;
        loop {
            // Check stop flag periodically
            if i % 100 == 0 && stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let mut curr_layout = accepted_layout.clone();
            let swaps = rand::random::<usize>() % num_swaps + 1;
            curr_layout.shuffle(swaps);

            // Pre-scoring constraints check
            if !self.is_layout_valid(&curr_layout) {
                i += 1;
                continue;
            }

            let (total_penalty, _) = cost::score_all(
                &self.quartads,
                &self.monograms,
                self.letters_len,
                &curr_layout,
                &self.config,
            );
            let scaled_penalty = total_penalty / self.letters_len as f64;

            // Use a cycling temperature schedule: reset every 15000 iterations
            // This helps maintain exploration in long runs
            let temp_iter = ((i - 1) % 15000) + 1;
            if crate::annealing::accept_transition(scaled_penalty - accepted_penalty, temp_iter) {
                accepted_layout = curr_layout.clone();
                accepted_penalty = scaled_penalty;
                accepted += 1;

                on_event(WorkerEvent::ChainAccepted {
                    worker_id: 0, // will be overridden by coordinator
                    iter: i as u64,
                    penalty: scaled_penalty,
                    layout: curr_layout.clone(),
                    seed,
                    swaps_used: swaps,
                });

                if scaled_penalty + 1e-12 < best_penalty {
                    best_penalty = scaled_penalty;
                    best_layout = curr_layout.clone();
                    on_event(WorkerEvent::ChainBest {
                        worker_id: 0,
                        iter: i as u64,
                        penalty: best_penalty,
                        layout: best_layout.clone(),
                        seed,
                        swaps_used: swaps,
                    });
                }
            }

            if i % 1000 == 0 {
                on_event(WorkerEvent::Progress {
                    worker_id: 0,
                    iter: i as u64,
                    accepted,
                });
            }

            i += 1;
        }
    }

    /// Run local refinement by exhaustively trying swaps
    pub fn refine(
        &self,
        initial_layout: Layout,
        max_depth: usize,
        top_n: usize,
        debug: bool,
    ) -> Vec<(Layout, f64, Vec<cost::PenaltyComponent>)> {
        let mut current_layout = initial_layout;
        let (initial_penalty, _) = cost::score_all(
            &self.quartads,
            &self.monograms,
            self.letters_len,
            &current_layout,
            &self.config,
        );
        let mut current_penalty = initial_penalty / self.letters_len as f64;

        println!("Initial penalty: {}", current_penalty);

        loop {
            let mut best_layouts: Vec<LayoutEntry> = Vec::new();
            let mut improved = false;

            // Try all possible swaps up to max_depth
            self.enumerate_swaps(&current_layout, max_depth, &mut best_layouts, top_n, debug);

            // Check if we found improvement
            if let Some(best) = best_layouts.first() {
                if best.penalty < current_penalty {
                    current_layout = best.layout.clone();
                    current_penalty = best.penalty;
                    improved = true;

                    if debug {
                        println!("Improved to penalty: {}", current_penalty);
                    }
                }
            }

            if !improved {
                break;
            }
        }

        // Return final result with details
        let (total, components) = cost::score_all(
            &self.quartads,
            &self.monograms,
            self.letters_len,
            &current_layout,
            &self.config,
        );
        vec![(current_layout, total / self.letters_len as f64, components)]
    }

    fn enumerate_swaps(
        &self,
        base_layout: &Layout,
        depth: usize,
        best_layouts: &mut Vec<LayoutEntry>,
        top_n: usize,
        debug: bool,
    ) {
        // For depth 1, try all single swaps
        if depth >= 1 {
            for i in 0..crate::geometry::NUM_KEYS {
                for j in (i + 1)..crate::geometry::NUM_KEYS {
                    let mut layout = base_layout.clone();
                    layout.swap(i, j);

                    self.evaluate_and_store(layout, best_layouts, top_n);
                }
            }
        }

        // For depth 2, try all pairs of swaps
        if depth >= 2 {
            for i1 in 0..crate::geometry::NUM_KEYS {
                for j1 in (i1 + 1)..crate::geometry::NUM_KEYS {
                    for i2 in 0..crate::geometry::NUM_KEYS {
                        for j2 in (i2 + 1)..crate::geometry::NUM_KEYS {
                            // Skip overlapping swaps
                            if i2 == i1 || i2 == j1 || j2 == i1 || j2 == j1 {
                                continue;
                            }

                            let mut layout = base_layout.clone();
                            layout.swap(i1, j1);
                            layout.swap(i2, j2);

                            self.evaluate_and_store(layout, best_layouts, top_n);
                        }
                    }
                }
            }
        }

        // For depth 3+, the combinatorics explode, so we limit to depth 2
        if depth > 2 && debug {
            println!("Note: Refinement limited to depth 2 for performance reasons");
        }
    }

    fn evaluate_and_store(
        &self,
        layout: Layout,
        best_layouts: &mut Vec<LayoutEntry>,
        top_n: usize,
    ) {
        // Hard constraints pre-check
        if !self.is_layout_valid(&layout) {
            return;
        }

        let (total_penalty, _) = cost::score_all(
            &self.quartads,
            &self.monograms,
            self.letters_len,
            &layout,
            &self.config,
        );
        let scaled_penalty = total_penalty / self.letters_len as f64;

        let entry = LayoutEntry {
            layout,
            penalty: scaled_penalty,
        };

        // Insert sorted
        let pos = best_layouts
            .iter()
            .position(|e| e.penalty > entry.penalty)
            .unwrap_or(best_layouts.len());
        best_layouts.insert(pos, entry);

        // Keep only top N
        if best_layouts.len() > top_n {
            best_layouts.pop();
        }
    }

    /// Print optimization results
    pub fn print_results(&self, results: &[(Layout, f64, Vec<cost::PenaltyComponent>)]) {
        use crate::geometry::Hand;
        for (i, (layout, scaled_penalty, components)) in results.iter().enumerate() {
            if i > 0 {
                println!();
            }

            println!("{}", layout);
            println!("Total penalty (scaled): {:.3}", scaled_penalty);

            // Compute hand usage
            if self.letters_len > 0 {
                let mut left = 0usize;
                for code in 0..26usize {
                    let letter = (b'a' + code as u8) as char;
                    if let Some(info) = layout.get_key_info(letter) {
                        if info.hand == Hand::Left {
                            left += self.monograms[code];
                        }
                    }
                }
                let p_left = left as f64 / self.letters_len as f64;
                let p_right = 1.0 - p_left;
                println!(
                    "Hand usage: L {:>5.1}% |{}| {:>5.1}% R",
                    100.0 * p_left,
                    balance_bar(p_left),
                    100.0 * p_right
                );
            }

            for component in components {
                if component.total.abs() > 0.001 {
                    println!("  {}: {:.1}", component.name, component.total);
                }
            }
        }
    }
}

fn balance_bar(p_left: f64) -> String {
    let width = 40usize;
    let left_chars = (p_left * width as f64).round() as usize;
    let right_chars = width - left_chars;
    format!("{}{}", "#".repeat(left_chars), ".".repeat(right_chars))
}
