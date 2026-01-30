use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use crate::coordinator::WorkerEvent;

pub struct WorkerHistory {
    worker_id: u32,
    writer: BufWriter<File>,
    last_accepted_penalty: Option<f64>,
    last_flush: Instant,
    flush_period_secs: u64,
}

impl WorkerHistory {
    pub fn new(dir: &Path, worker_id: u32, flush_period_secs: u64) -> std::io::Result<Self> {
        let path = dir.join(format!("events_worker_{}.csv", worker_id));
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;

        let mut writer = BufWriter::new(file);
        // Write CSV header
        writeln!(
            writer,
            "ts,worker_id,iter,event_type,penalty,swaps_used,delta_to_prev"
        )?;
        writer.flush()?;

        Ok(WorkerHistory {
            worker_id,
            writer,
            last_accepted_penalty: None,
            last_flush: Instant::now(),
            flush_period_secs,
        })
    }

    pub fn log_event(&mut self, event: &WorkerEvent) -> std::io::Result<()> {
        let ts = crate::results::now_epoch();

        match event {
            WorkerEvent::ChainAccepted {
                iter,
                penalty,
                swaps_used,
                ..
            } => {
                let delta = self
                    .last_accepted_penalty
                    .map(|last| penalty - last)
                    .unwrap_or(0.0);

                writeln!(
                    self.writer,
                    "{},{},{},accepted,{:.6},{},{}",
                    ts, self.worker_id, iter, penalty, swaps_used, delta
                )?;

                self.last_accepted_penalty = Some(*penalty);
            }
            WorkerEvent::ChainBest {
                iter,
                penalty,
                swaps_used,
                ..
            } => {
                let delta = self
                    .last_accepted_penalty
                    .map(|last| penalty - last)
                    .unwrap_or(0.0);

                writeln!(
                    self.writer,
                    "{},{},{},best,{:.6},{},{}",
                    ts, self.worker_id, iter, penalty, swaps_used, delta
                )?;
            }
            WorkerEvent::Progress { .. } => {
                // Don't log progress events to CSV
            }
        }

        // Flush periodically
        if self.last_flush.elapsed().as_secs() >= self.flush_period_secs {
            self.writer.flush()?;
            self.last_flush = Instant::now();
        }

        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub struct HistoryManager {
    workers: HashMap<u32, WorkerHistory>,
    dir: std::path::PathBuf,
    flush_period_secs: u64,
}

impl HistoryManager {
    pub fn new(dir: &Path, flush_period_secs: u64) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(HistoryManager {
            workers: HashMap::new(),
            dir: dir.to_path_buf(),
            flush_period_secs,
        })
    }

    pub fn log_event(&mut self, event: &WorkerEvent) -> std::io::Result<()> {
        let worker_id = match event {
            WorkerEvent::ChainAccepted { worker_id, .. } => *worker_id,
            WorkerEvent::ChainBest { worker_id, .. } => *worker_id,
            WorkerEvent::Progress { worker_id, .. } => *worker_id,
        };

        // Create worker history on demand
        if !self.workers.contains_key(&worker_id) {
            let history = WorkerHistory::new(&self.dir, worker_id, self.flush_period_secs)?;
            self.workers.insert(worker_id, history);
        }

        self.workers.get_mut(&worker_id).unwrap().log_event(event)
    }

    pub fn flush_all(&mut self) -> std::io::Result<()> {
        for worker in self.workers.values_mut() {
            worker.flush()?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct SwapProfile {
    pub swap_counts: HashMap<usize, SwapStats>,
}

#[derive(Default, Clone)]
pub struct SwapStats {
    pub accepted_count: u64,
    pub total_delta: f64,
    pub min_delta: f64,
    pub max_delta: f64,
}

impl SwapStats {
    pub fn update(&mut self, delta: f64) {
        self.accepted_count += 1;
        self.total_delta += delta;

        if self.accepted_count == 1 {
            self.min_delta = delta;
            self.max_delta = delta;
        } else {
            self.min_delta = self.min_delta.min(delta);
            self.max_delta = self.max_delta.max(delta);
        }
    }

    pub fn mean_delta(&self) -> f64 {
        if self.accepted_count > 0 {
            self.total_delta / self.accepted_count as f64
        } else {
            0.0
        }
    }
}

pub fn analyze_events_dir(dir: &Path) -> std::io::Result<SwapProfile> {
    let mut profile = SwapProfile::default();

    // Read all CSV files in directory
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("csv")
            && path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("events_worker_"))
                .unwrap_or(false)
        {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);

            // Skip header
            let mut lines = reader.lines();
            lines.next(); // Skip header

            for line in lines {
                let line = line?;
                let parts: Vec<&str> = line.split(',').collect();

                if parts.len() >= 7 {
                    let event_type = parts[3];
                    if event_type == "accepted" {
                        if let (Ok(swaps_used), Ok(delta)) =
                            (parts[5].parse::<usize>(), parts[6].parse::<f64>())
                        {
                            profile
                                .swap_counts
                                .entry(swaps_used)
                                .or_default()
                                .update(delta);
                        }
                    }
                }
            }
        }
    }

    Ok(profile)
}

pub fn print_swap_profile(profile: &SwapProfile) {
    println!("Swap Count Analysis:");
    println!("===================");
    println!();
    println!("Swaps | Accepted | Mean Delta | Min Delta | Max Delta");
    println!("------+----------+------------+-----------+----------");

    let mut swap_nums: Vec<_> = profile.swap_counts.keys().cloned().collect();
    swap_nums.sort();

    for swaps in &swap_nums {
        let stats = &profile.swap_counts[swaps];
        println!(
            "{:5} | {:8} | {:10.6} | {:9.6} | {:9.6}",
            swaps,
            stats.accepted_count,
            stats.mean_delta(),
            stats.min_delta,
            stats.max_delta
        );
    }

    // ASCII bar chart
    println!();
    println!("Acceptance Rate by Swap Count:");
    println!();

    if let Some(max_count) = profile.swap_counts.values().map(|s| s.accepted_count).max() {
        let bar_width = 40;

        for swaps in &swap_nums {
            let stats = &profile.swap_counts[swaps];
            let bar_len =
                ((stats.accepted_count as f64 / max_count as f64) * bar_width as f64) as usize;
            let bar = "#".repeat(bar_len);

            println!("{} swaps: {} {}", swaps, bar, stats.accepted_count);
        }
    }
}
