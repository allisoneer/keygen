// Remove nightly feature requirement
mod annealing;
mod constraints;
mod coordinator;
mod corpus;
mod cost;
mod geometry;
mod history;
mod layout_26;
mod optimizer;
mod results;
mod symmetry;

extern crate getopts;
extern crate num_cpus;
extern crate rand;

use getopts::Options;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn main() {
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("d", "debug", "show debug logging");
    opts.optopt(
        "t",
        "top",
        "number of top layouts to print (default: 1)",
        "TOP_LAYOUTS",
    );
    opts.optopt(
        "s",
        "swaps",
        "maximum number of swaps per iteration (default: 3)",
        "SWAPS",
    );
    opts.optopt(
        "i",
        "init-layout",
        "initial layout string (26 letters)",
        "LAYOUT",
    );

    // Options for run-par
    opts.optopt(
        "",
        "threads",
        "number of worker threads (default: num_cpus)",
        "N",
    );
    opts.optopt(
        "",
        "time",
        "time limit (e.g., 5m, 2h) (default: unlimited)",
        "TIME",
    );
    opts.optopt("", "persist", "directory for persistence", "DIR");
    opts.optopt("", "repo-cap", "repository capacity (default: 100)", "K");
    opts.optopt("", "seed", "random seed", "SEED");
    opts.optopt(
        "",
        "target-penalty",
        "stop when penalty reaches this value",
        "PENALTY",
    );

    // Options for results commands
    opts.optopt("", "events", "directory containing event CSV files", "DIR");
    opts.optopt("", "file", "file to read results from", "FILE");
    opts.optopt("", "out", "output file for merge/snapshot", "FILE");
    opts.optopt("", "jsonl", "JSONL file to convert to snapshot", "FILE");
    opts.optflag("", "force", "force merge even with different fingerprints");

    let args: Vec<String> = env::args().collect();
    let progname = &args[0];

    if args.len() < 2 {
        print_usage(progname, opts);
        return;
    }

    let command = &args[1];
    let matches = match opts.parse(&args[2..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("Error: {}", f);
            std::process::exit(1);
        }
    };

    if matches.opt_present("h") {
        print_usage(progname, opts);
        return;
    }

    // Handle results command specially (doesn't need corpus)
    if command == "results" {
        handle_results_command(&matches);
        return;
    }

    // Read corpus for other commands
    let corpus_filename = match matches.free.first() {
        Some(f) => f,
        None => {
            print_usage(progname, opts);
            return;
        }
    };

    let mut corpus = String::new();
    match File::open(corpus_filename) {
        Ok(mut f) => {
            if let Err(e) = f.read_to_string(&mut corpus) {
                eprintln!("Error reading corpus: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error opening corpus file: {}", e);
            std::process::exit(1);
        }
    }

    // Parse options
    let debug = matches.opt_present("d");
    let top = parse_num_opt(matches.opt_str("t"), 1usize);
    let swaps = parse_num_opt(matches.opt_str("s"), 3usize);

    // Get initial layout
    let initial_layout = if let Some(layout_str) = matches.opt_str("i") {
        match layout_26::Layout::from_string(&layout_str) {
            Ok(layout) => layout,
            Err(e) => {
                eprintln!("Error parsing initial layout: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        layout_26::Layout::alphabetical()
    };

    // Create optimizer with default config
    let config = cost::Config::default();

    // Hard constraints: MVP forbids "you" on one hand
    let constraints = constraints::Constraints::with_forbid_same_hand_words(&["you"]);

    let opt = optimizer::Optimizer::new_with_constraints(&corpus, config, constraints);

    match command.as_str() {
        "run-par" => {
            println!("Running parallel simulated annealing optimization...");
            println!("Corpus length: {} characters", corpus.len());

            // Parse parallel-specific options
            let threads = parse_num_opt(matches.opt_str("threads"), num_cpus::get());
            let persist_dir = matches.opt_str("persist");
            let repo_cap = parse_num_opt(matches.opt_str("repo-cap"), 100usize);
            let seed = parse_num_opt(matches.opt_str("seed"), rand::random::<u64>());
            let target_penalty = matches
                .opt_str("target-penalty")
                .and_then(|s| s.parse::<f64>().ok());

            // Parse time limit
            let time_seconds = matches.opt_str("time").and_then(|t| parse_duration(&t));

            println!("Threads: {}", threads);
            if let Some(ref dir) = persist_dir {
                println!("Persist directory: {}", dir);
            }
            if let Some(secs) = time_seconds {
                println!("Time limit: {}s", secs);
            }
            println!("Repository capacity: {}", repo_cap);
            println!("Seed: {}", seed);
            println!();

            // Create config
            let config = coordinator::RunnerConfig {
                threads,
                repo_capacity: repo_cap,
                flush_period_secs: 30,
                num_swaps: swaps,
                persist_dir,
                log_sample_rate: 0.01,
            };

            let stop_condition = coordinator::StopCondition {
                max_seconds: time_seconds,
                max_iters: None,
                target_penalty,
            };

            let opt_arc = std::sync::Arc::new(opt);

            // Run parallel optimization
            match coordinator::run_parallel(opt_arc.clone(), config, stop_condition, seed) {
                Ok(repo) => {
                    println!("\nOptimization complete!");
                    println!("Final repository size: {}", repo.len());

                    // Print top results with symmetric grouping
                    let top_results = repo.top();
                    if !top_results.is_empty() {
                        println!(
                            "\nTop {} results with symmetric layouts:",
                            top_results.len().min(5)
                        );
                        println!();
                        let records: Vec<_> = top_results.into_iter().cloned().collect();
                        results::print_symmetric_results(&records, &opt_arc, 5);
                    }
                }
                Err(e) => {
                    eprintln!("Error during parallel optimization: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "run" => {
            println!("Running simulated annealing optimization...");
            println!("Corpus length: {} characters", corpus.len());
            println!();

            loop {
                let results = opt.anneal(initial_layout.clone(), swaps, top, debug);
                println!();
                println!("Top {} layouts:", results.len());
                println!();
                opt.print_results(&results);
                println!("\n{}\n", "=".repeat(60));
            }
        }
        "refine" => {
            println!("Running local refinement...");
            println!("Corpus length: {} characters", corpus.len());
            println!();

            let results = opt.refine(initial_layout, swaps, top, debug);
            println!();
            println!("Refined layout:");
            println!();
            opt.print_results(&results);
        }
        _ => {
            print_usage(progname, opts);
        }
    }
}

fn handle_results_command(matches: &getopts::Matches) {
    // Handle results subcommands
    if matches.free.is_empty() {
        eprintln!("Error: results command requires a subcommand");
        eprintln!("Available subcommands: show, merge, snapshot, analyze-swaps");
        std::process::exit(1);
    }

    let subcommand = &matches.free[0];
    match subcommand.as_str() {
        "show" => {
            let file_path = matches.opt_str("file");
            if file_path.is_none() {
                eprintln!("Error: --file option is required for results show");
                std::process::exit(1);
            }

            let top = parse_num_opt(matches.opt_str("t"), 10usize);

            match results::load_snapshot(Path::new(&file_path.unwrap())) {
                Ok(records) => {
                    if records.is_empty() {
                        println!("No results found in file");
                    } else {
                        // Create a minimal optimizer just for printing
                        // In a real implementation, we'd store corpus/config info
                        println!("Loaded {} results from file", records.len());
                        println!();

                        // Just print the layouts without penalty verification for now
                        for (i, rec) in records.iter().take(top).enumerate() {
                            if i > 0 {
                                println!();
                            }

                            println!("=== Layout #{} (Penalty: {:.6}) ===", i + 1, rec.penalty);
                            println!("Canonical key: {}", rec.canonical);
                            println!();

                            if let Ok(layout) = layout_26::Layout::from_string(&rec.layout) {
                                println!("Original:");
                                println!("{}", layout);
                            }

                            println!();
                            if let Ok(mirrored) = layout_26::Layout::from_string(&rec.mirrored) {
                                println!("Mirrored:");
                                println!("{}", mirrored);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error loading results: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "analyze-swaps" => {
            let events_dir = matches.opt_str("events").unwrap_or_else(|| ".".to_string());

            println!("Analyzing swap events from: {}", events_dir);
            match history::analyze_events_dir(Path::new(&events_dir)) {
                Ok(profile) => {
                    println!();
                    history::print_swap_profile(&profile);
                }
                Err(e) => {
                    eprintln!("Error analyzing events: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "merge" => {
            let out_path = matches.opt_str("out");
            if out_path.is_none() {
                eprintln!("Error: --out option is required for results merge");
                std::process::exit(1);
            }

            let force = matches.opt_present("force");
            let capacity = parse_num_opt(matches.opt_str("repo-cap"), 100usize);

            // Collect input files from remaining free args
            let input_files: Vec<&str> = matches.free[1..].iter().map(|s| s.as_str()).collect();

            if input_files.is_empty() {
                eprintln!("Error: No input files specified for merge");
                eprintln!("Usage: results merge --out merged.jsonl file1.jsonl file2.json ...");
                std::process::exit(1);
            }

            println!(
                "Merging {} files into {}",
                input_files.len(),
                out_path.as_ref().unwrap()
            );

            match results::merge_files(&input_files, capacity, force) {
                Ok((repo, warnings)) => {
                    for warning in warnings {
                        eprintln!("{}", warning);
                    }

                    let records = repo.top().into_iter().cloned().collect::<Vec<_>>();
                    println!("Merged {} unique results", records.len());

                    // Write output
                    let out_file = out_path.unwrap();
                    let out_path = Path::new(&out_file);
                    if out_path.extension().and_then(|s| s.to_str()) == Some("json") {
                        match results::snapshot_best(out_path, &records) {
                            Ok(_) => println!("Wrote snapshot to {:?}", out_path),
                            Err(e) => {
                                eprintln!("Error writing output: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        // Write JSONL
                        for rec in &records {
                            if let Err(e) = results::append_jsonl(out_path, rec) {
                                eprintln!("Error writing record: {}", e);
                                std::process::exit(1);
                            }
                        }
                        println!("Wrote {} records to {:?}", records.len(), out_path);
                    }
                }
                Err(e) => {
                    eprintln!("Error merging files: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "snapshot" => {
            let jsonl_path = matches.opt_str("jsonl");
            let out_path = matches.opt_str("out");

            if jsonl_path.is_none() || out_path.is_none() {
                eprintln!("Error: --jsonl and --out options are required for results snapshot");
                std::process::exit(1);
            }

            let top = parse_num_opt(matches.opt_str("t"), 100usize);

            println!("Converting JSONL to snapshot...");
            match results::load_jsonl_iter(Path::new(&jsonl_path.unwrap())) {
                Ok(iter) => {
                    let records: Vec<_> = iter.take(top).collect();
                    println!("Loaded {} records", records.len());

                    let out_file = out_path.unwrap();
                    match results::snapshot_best(Path::new(&out_file), &records) {
                        Ok(_) => println!("Wrote snapshot to {}", out_file),
                        Err(e) => {
                            eprintln!("Error writing snapshot: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error loading JSONL: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown results subcommand: {}", subcommand);
            eprintln!("Available subcommands: show, merge, snapshot, analyze-swaps");
            std::process::exit(1);
        }
    }
}

fn print_usage(progname: &str, opts: Options) {
    let brief = format!(
        "Usage: {} (run|run-par|refine|results) <corpus/subcommand> [OPTIONS]",
        progname
    );
    print!("{}", opts.usage(&brief));
    println!();
    println!("Commands:");
    println!("  run                    - Run simulated annealing optimization (loops forever)");
    println!("  run-par                - Run parallel simulated annealing with persistence");
    println!("  refine                 - Run local refinement on initial layout");
    println!("  results show           - Display saved results with symmetric layouts");
    println!("  results merge          - Merge multiple result files");
    println!("  results snapshot       - Convert JSONL to JSON snapshot");
    println!("  results analyze-swaps  - Analyze swap acceptance patterns from event logs");
    println!();
    println!("Examples:");
    println!("  {} run corpus.txt --swaps 3 --top 5", progname);
    println!(
        "  {} run-par corpus.txt --threads 8 --time 2h --persist ./results",
        progname
    );
    println!(
        "  {} results show --file ./results/best.json --top 5",
        progname
    );
    println!(
        "  {} results merge --out merged.jsonl result1.jsonl result2.json",
        progname
    );
    println!(
        "  {} results snapshot --jsonl results.jsonl --out best.json --top 100",
        progname
    );
    println!("  {} results analyze-swaps --events ./results", progname);
}

fn parse_duration(s: &str) -> Option<u64> {
    // Simple duration parser supporting formats like "5m", "2h", "30s"
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_part, unit) = if let Some(stripped) = s.strip_suffix('h') {
        (stripped, 'h')
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, 'm')
    } else if let Some(stripped) = s.strip_suffix('s') {
        (stripped, 's')
    } else {
        // Default to seconds if no unit
        (s, 's')
    };

    let num = num_part.parse::<u64>().ok()?;

    match unit {
        'h' => Some(num * 3600),
        'm' => Some(num * 60),
        's' => Some(num),
        _ => None,
    }
}

fn parse_num_opt<T>(s: Option<String>, default: T) -> T
where
    T: std::str::FromStr + std::fmt::Display,
{
    match s {
        None => default,
        Some(num) => match num.parse::<T>() {
            Ok(n) => n,
            Err(_) => {
                eprintln!(
                    "Warning: Invalid numeric value '{}'. Using default: {}",
                    num, default
                );
                default
            }
        },
    }
}
