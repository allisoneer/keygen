use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use flume;

use crate::history::HistoryManager;
use crate::layout_26::Layout;
use crate::optimizer::Optimizer;
use crate::results::{
    append_jsonl, config_fingerprint, corpus_fingerprint, snapshot_best, BestRepo, ResultRecord,
};
use crate::symmetry::{canonical_key, layout_compact_string, mirror_layout};

#[derive(Clone)]
pub enum WorkerEvent {
    ChainAccepted {
        worker_id: u32,
        iter: u64,
        penalty: f64,
        layout: Layout,
        seed: u64,
        swaps_used: usize,
    },
    ChainBest {
        worker_id: u32,
        iter: u64,
        penalty: f64,
        layout: Layout,
        seed: u64,
        swaps_used: usize,
    },
    Progress {
        worker_id: u32,
        iter: u64,
        accepted: u64,
    },
}

#[derive(Clone)]
pub struct StopCondition {
    pub max_seconds: Option<u64>,
    pub max_iters: Option<u64>,
    pub target_penalty: Option<f64>,
}

pub struct RunnerConfig {
    pub threads: usize,
    pub repo_capacity: usize,
    pub flush_period_secs: u64,
    pub num_swaps: usize,
    pub persist_dir: Option<String>,
    pub log_sample_rate: f64,
}

pub fn run_parallel(
    optimizer: Arc<Optimizer>,
    config: RunnerConfig,
    stop_condition: StopCondition,
    initial_seed: u64,
) -> io::Result<BestRepo> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let (tx, rx) = flume::bounded(1024);

    // Set up signal handler for graceful shutdown
    let stop_flag_ctrlc = stop_flag.clone();
    ctrlc::set_handler(move || {
        eprintln!("\nReceived interrupt signal. Shutting down gracefully...");
        stop_flag_ctrlc.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    // Create BestRepo
    let mut repo = BestRepo::new(config.repo_capacity);

    // Load existing results if resuming
    let mut initial_best_penalty = f64::INFINITY;
    if let Some(ref dir) = config.persist_dir {
        let snapshot_path = Path::new(dir).join("best.json");
        if snapshot_path.exists() {
            println!("Loading existing results from {:?}", snapshot_path);
            match crate::results::load_snapshot(&snapshot_path) {
                Ok(records) => {
                    // Track the best penalty from loaded results
                    for rec in &records {
                        if rec.penalty < initial_best_penalty {
                            initial_best_penalty = rec.penalty;
                        }
                        repo.insert(rec.clone());
                    }
                    println!("Loaded {} existing results", repo.len());
                    if initial_best_penalty < f64::INFINITY {
                        println!("Previous best penalty: {:.6}", initial_best_penalty);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not load existing results: {}", e);
                }
            }
        }
    }

    // Get fingerprints
    let cfg_fingerprint = config_fingerprint(optimizer.get_config());
    let corpus_fingerprint = corpus_fingerprint(
        optimizer.get_quartads(),
        optimizer.get_monograms(),
        optimizer.get_letters_len(),
    );

    // Spawn worker threads
    let mut handles = vec![];
    for worker_id in 0..config.threads {
        let tx = tx.clone();
        let optimizer = optimizer.clone();
        let stop_flag = stop_flag.clone();
        let seed = initial_seed + worker_id as u64;
        let num_swaps = config.num_swaps;

        let handle = thread::spawn(move || {
            // Use seed to initialize layout differently for each worker
            let mut layout = Layout::alphabetical();
            layout.shuffle(worker_id as usize * 5 + 3); // Different initial shuffle per worker

            optimizer.anneal_streaming(layout, num_swaps, seed, stop_flag.clone(), |event| {
                // Update worker_id in the event
                let event_with_id = match event {
                    WorkerEvent::ChainAccepted {
                        iter,
                        penalty,
                        layout,
                        seed,
                        swaps_used,
                        ..
                    } => WorkerEvent::ChainAccepted {
                        worker_id: worker_id as u32,
                        iter,
                        penalty,
                        layout,
                        seed,
                        swaps_used,
                    },
                    WorkerEvent::ChainBest {
                        iter,
                        penalty,
                        layout,
                        seed,
                        swaps_used,
                        ..
                    } => WorkerEvent::ChainBest {
                        worker_id: worker_id as u32,
                        iter,
                        penalty,
                        layout,
                        seed,
                        swaps_used,
                    },
                    WorkerEvent::Progress { iter, accepted, .. } => WorkerEvent::Progress {
                        worker_id: worker_id as u32,
                        iter,
                        accepted,
                    },
                };

                match &event_with_id {
                    WorkerEvent::ChainAccepted { .. } => {
                        // Sample accepted events based on log_sample_rate
                        // For now, always send (implement sampling later)
                        let _ = tx.send(event_with_id);
                    }
                    _ => {
                        // Always send chain bests and progress
                        let _ = tx.send(event_with_id);
                    }
                }
            });
        });
        handles.push(handle);
    }

    // Drop the original sender so receiver will end when all workers finish
    drop(tx);

    // Aggregator thread
    let start_time = Instant::now();
    let mut last_flush = Instant::now();
    let mut best_penalty = initial_best_penalty;
    let mut _total_accepted = 0u64;
    let mut total_iters = 0u64;

    // Create persist directory if needed
    let mut history_manager = if let Some(ref dir) = config.persist_dir {
        std::fs::create_dir_all(dir)?;
        Some(HistoryManager::new(Path::new(dir), 5)?)
    } else {
        None
    };

    // Process events
    while let Ok(event) = rx.recv() {
        // Log to history
        if let Some(ref mut history) = history_manager {
            history.log_event(&event)?;
        }
        match event {
            WorkerEvent::ChainBest {
                worker_id,
                iter,
                penalty,
                layout,
                seed,
                ..
            } => {
                let rec = ResultRecord {
                    layout: layout_compact_string(&layout),
                    mirrored: layout_compact_string(&mirror_layout(&layout)),
                    canonical: canonical_key(&layout),
                    penalty,
                    components: vec![], // Empty in streaming mode
                    seed,
                    worker_id,
                    iter,
                    ts: crate::results::now_epoch(),
                    corpus_fingerprint: corpus_fingerprint.clone(),
                    config_fingerprint: cfg_fingerprint.clone(),
                };

                if let Some(_evicted) = repo.insert(rec.clone()) {
                    // A record was evicted
                }

                if penalty < best_penalty {
                    best_penalty = penalty;
                    println!(
                        "New best: {:.6} (worker {}, iter {})",
                        penalty, worker_id, iter
                    );
                }

                // Append to JSONL
                if let Some(ref dir) = config.persist_dir {
                    let jsonl_path = Path::new(dir).join("results.jsonl");
                    append_jsonl(&jsonl_path, &rec)?;
                }
            }
            WorkerEvent::ChainAccepted { .. } => {
                // These are sampled, just count them
                _total_accepted += 1;
            }
            WorkerEvent::Progress { iter, .. } => {
                total_iters = total_iters.max(iter);
            }
        }

        // Check stop conditions
        if let Some(target) = stop_condition.target_penalty {
            if best_penalty <= target {
                println!("Reached target penalty {}", target);
                stop_flag.store(true, Ordering::Relaxed);
            }
        }

        if let Some(max_secs) = stop_condition.max_seconds {
            if start_time.elapsed().as_secs() >= max_secs {
                println!("Time limit reached");
                stop_flag.store(true, Ordering::Relaxed);
            }
        }

        if let Some(max_iters) = stop_condition.max_iters {
            if total_iters >= max_iters {
                println!("Iteration limit reached");
                stop_flag.store(true, Ordering::Relaxed);
            }
        }

        // Periodic flush
        if last_flush.elapsed().as_secs() >= config.flush_period_secs {
            if let Some(ref dir) = config.persist_dir {
                let snapshot_path = Path::new(dir).join("best.json");
                let top_results: Vec<ResultRecord> =
                    repo.top().into_iter().map(|r| r.clone()).collect();
                snapshot_best(&snapshot_path, &top_results)?;

                println!("\n--- Progress Update ---");
                println!("Time elapsed: {:.1}s", start_time.elapsed().as_secs_f64());
                println!("Best penalty: {:.6}", best_penalty);
                println!("Repository size: {}", repo.len());
                println!("Top 5 layouts:");
                for (i, rec) in top_results.iter().take(5).enumerate() {
                    println!("  {}. {:.6} - {}", i + 1, rec.penalty, &rec.layout[0..10]);
                }
                println!("----------------------\n");
            }
            last_flush = Instant::now();
        }
    }

    // Wait for all workers to finish
    for handle in handles {
        handle.join().expect("Worker thread panicked");
    }

    // Final snapshot
    if let Some(ref dir) = config.persist_dir {
        let snapshot_path = Path::new(dir).join("best.json");
        let top_results: Vec<ResultRecord> = repo.top().into_iter().cloned().collect();
        snapshot_best(&snapshot_path, &top_results)?;
        println!("Final results saved to {:?}", snapshot_path);

        // Flush history
        if let Some(ref mut history) = history_manager {
            history.flush_all()?;
        }
    }

    Ok(repo)
}

// Simple signal handler stub for now
mod ctrlc {
    pub fn set_handler<F>(_handler: F) -> Result<(), &'static str>
    where
        F: Fn() + Send + 'static,
    {
        // In a real implementation, would register signal handler here
        // For now, just return Ok
        Ok(())
    }
}
