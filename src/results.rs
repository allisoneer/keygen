use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cost::Config;
use blake3::Hasher;

#[derive(Serialize, Deserialize, Clone)]
pub struct ResultRecord {
    pub layout: String,                 // 26 letters
    pub mirrored: String,               // 26 letters
    pub canonical: String,              // min(layout, mirrored)
    pub penalty: f64,                   // scaled penalty
    pub components: Vec<(String, f64)>, // optional, may be empty in streaming mode
    pub seed: u64,
    pub worker_id: u32,
    pub iter: u64, // annealing iteration when achieved
    pub ts: u64,   // epoch seconds
    pub corpus_fingerprint: String,
    pub config_fingerprint: String,
}

pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub struct BestRepo {
    pub capacity: usize,
    by_key: BTreeMap<String, ResultRecord>, // canonical -> record
    ordered: BTreeSet<(i64 /* milli-penalty */, String /* canonical */)>,
}

impl BestRepo {
    pub fn new(capacity: usize) -> Self {
        BestRepo {
            capacity,
            by_key: BTreeMap::new(),
            ordered: BTreeSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    #[allow(dead_code)] // part of public API for completeness with len()
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    pub fn insert(&mut self, rec: ResultRecord) -> Option<ResultRecord> {
        // Order by rounded milli penalty for stable ordering and canonical tie-breaker
        let milli = (rec.penalty * 1000.0).round() as i64;
        if let Some(existing) = self.by_key.get(&rec.canonical) {
            if rec.penalty + 1e-9 >= existing.penalty {
                return None;
            }
            // Replace existing better record
            let old_milli = (existing.penalty * 1000.0).round() as i64;
            self.ordered
                .remove(&(old_milli, existing.canonical.clone()));
        }
        self.by_key.insert(rec.canonical.clone(), rec.clone());
        self.ordered.insert((milli, rec.canonical.clone()));

        // Evict worst if over capacity
        if self.by_key.len() > self.capacity {
            if let Some(worst_key) = self.ordered.iter().next_back().cloned() {
                self.ordered.remove(&worst_key);
                let (_, canonical) = worst_key;
                return self.by_key.remove(&canonical);
            }
        }
        None
    }

    pub fn top(&self) -> Vec<&ResultRecord> {
        self.ordered
            .iter()
            .take(self.capacity)
            .map(|(_, k)| self.by_key.get(k).unwrap())
            .collect()
    }
}

pub fn config_fingerprint(cfg: &Config) -> String {
    let mut h = Hasher::new();
    macro_rules! feed {
        ($($f:expr),* $(,)?) => { $(h.update(&format!("{:.6}", $f).as_bytes());)* };
    }
    feed!(
        cfg.base_weight,
        cfg.same_finger_bigram,
        cfg.same_hand_bigram,
        cfg.alt_hand_bigram,
        cfg.movement_penalty_base,
        cfg.extreme_movement_threshold,
        cfg.extreme_movement_extra,
        cfg.pinky_ring_twist,
        cfg.roll_reversal,
        cfg.same_hand_4,
        cfg.alternating_hand_4,
        cfg.roll_out,
        cfg.roll_in,
        cfg.sandwich_distance_penalty,
        cfg.twist,
        cfg.same_hand_finger_repeat,
        cfg.broken_roll,
        cfg.balance_mild_start,
        cfg.balance_moderate_start,
        cfg.balance_severe_start,
        cfg.balance_mild_weight,
        cfg.balance_moderate_weight,
        cfg.balance_severe_weight,
        cfg.balance_severe_exponent
    );
    h.finalize().to_hex().to_string()
}

pub fn corpus_fingerprint(
    quartads: &std::collections::HashMap<[u8; 4], usize>,
    monograms: &[usize; 26],
    letters_len: usize,
) -> String {
    let mut h = Hasher::new();
    let mut keys: Vec<[u8; 4]> = quartads.keys().cloned().collect();
    keys.sort_unstable();
    for k in keys {
        h.update(&k);
        let v = quartads[&k] as u64;
        h.update(&v.to_le_bytes());
    }
    for &m in monograms.iter() {
        let v = m as u64;
        h.update(&v.to_le_bytes());
    }
    h.update(&(letters_len as u64).to_le_bytes());
    h.finalize().to_hex().to_string()
}

pub fn append_jsonl(path: &Path, record: &ResultRecord) -> std::io::Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "{}", serde_json::to_string(record)?)?;
    writer.flush()?;
    Ok(())
}

pub fn load_jsonl_iter(path: &Path) -> std::io::Result<impl Iterator<Item = ResultRecord>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    Ok(reader.lines().filter_map(|line| {
        line.ok()
            .and_then(|l| serde_json::from_str::<ResultRecord>(&l).ok())
    }))
}

pub fn snapshot_best(path: &Path, records: &[ResultRecord]) -> std::io::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &records)?;
    Ok(())
}

pub fn load_snapshot(path: &Path) -> std::io::Result<Vec<ResultRecord>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

pub fn merge_records<I: Iterator<Item = ResultRecord>>(records: I, capacity: usize) -> BestRepo {
    let mut repo = BestRepo::new(capacity);
    for rec in records {
        repo.insert(rec);
    }
    repo
}

pub fn merge_files(
    paths: &[&str],
    capacity: usize,
    force: bool,
) -> std::io::Result<(BestRepo, Vec<String>)> {
    let mut all_records = Vec::new();
    let mut warnings = Vec::new();
    let mut fingerprints = std::collections::HashSet::new();

    // Load all records and check fingerprints
    for path in paths {
        let path = Path::new(path);
        let records: Vec<ResultRecord> =
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                load_snapshot(path)?
            } else {
                // Assume JSONL
                load_jsonl_iter(path)?.collect()
            };

        for rec in records {
            // Collect unique fingerprint pairs
            let fp_pair = (
                rec.corpus_fingerprint.clone(),
                rec.config_fingerprint.clone(),
            );
            fingerprints.insert(fp_pair);
            all_records.push(rec);
        }
    }

    // Check fingerprint consistency
    if fingerprints.len() > 1 {
        if !force {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Found {} different corpus/config combinations. Use --force to merge anyway.",
                    fingerprints.len()
                ),
            ));
        } else {
            warnings.push(format!(
                "Warning: Merging results from {} different corpus/config combinations",
                fingerprints.len()
            ));
        }
    }

    // Merge all records
    let repo = merge_records(all_records.into_iter(), capacity);
    Ok((repo, warnings))
}

pub fn print_symmetric_results(
    records: &[ResultRecord],
    optimizer: &crate::optimizer::Optimizer,
    top_n: usize,
) {
    use crate::layout_26::Layout;

    for (i, rec) in records.iter().take(top_n).enumerate() {
        if i > 0 {
            println!();
        }

        // Parse layouts
        let layout = match Layout::from_string(&rec.layout) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error parsing layout: {}", e);
                continue;
            }
        };

        let mirrored = match Layout::from_string(&rec.mirrored) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error parsing mirrored layout: {}", e);
                continue;
            }
        };

        println!("=== Layout #{} (Penalty: {:.6}) ===", i + 1, rec.penalty);
        println!();

        // Print original layout
        println!("Original:");
        println!("{}", layout);

        // Print mirrored layout
        println!();
        println!("Mirrored:");
        println!("{}", mirrored);

        // Verify penalties match
        let (orig_penalty, orig_components) = crate::cost::score_all(
            optimizer.get_quartads(),
            optimizer.get_monograms(),
            optimizer.get_letters_len(),
            &layout,
            optimizer.get_config(),
        );
        let orig_scaled = orig_penalty / optimizer.get_letters_len() as f64;

        let (mirror_penalty, _) = crate::cost::score_all(
            optimizer.get_quartads(),
            optimizer.get_monograms(),
            optimizer.get_letters_len(),
            &mirrored,
            optimizer.get_config(),
        );
        let mirror_scaled = mirror_penalty / optimizer.get_letters_len() as f64;

        if (orig_scaled - mirror_scaled).abs() > 1e-6 {
            println!();
            println!(
                "WARNING: Penalty mismatch! Original: {:.6}, Mirrored: {:.6}",
                orig_scaled, mirror_scaled
            );
        }

        // Print component breakdown for original
        println!();
        println!("Penalty breakdown:");
        for comp in &orig_components {
            if comp.total.abs() > 0.001 {
                println!("  {}: {:.3}", comp.name, comp.total);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_26::Layout;
    use crate::symmetry::{canonical_key, layout_compact_string, mirror_layout};

    #[test]
    fn test_best_repo_ordering() {
        let mut repo = BestRepo::new(3);

        let layout1 = Layout::alphabetical();
        let rec1 = ResultRecord {
            layout: layout_compact_string(&layout1),
            mirrored: layout_compact_string(&mirror_layout(&layout1)),
            canonical: canonical_key(&layout1),
            penalty: 10.0,
            components: vec![],
            seed: 1,
            worker_id: 0,
            iter: 100,
            ts: now_epoch(),
            corpus_fingerprint: "test".to_string(),
            config_fingerprint: "test".to_string(),
        };

        repo.insert(rec1.clone());
        assert_eq!(repo.len(), 1);

        // Insert better record with same canonical key
        let mut rec2 = rec1.clone();
        rec2.penalty = 9.0;
        repo.insert(rec2.clone());
        assert_eq!(repo.len(), 1); // Should replace, not add

        // Insert worse record - should be ignored
        let mut rec3 = rec1.clone();
        rec3.penalty = 11.0;
        repo.insert(rec3);
        assert_eq!(repo.len(), 1);

        // Verify top record has best penalty
        let top = repo.top();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].penalty, 9.0);
    }

    #[test]
    fn test_best_repo_capacity_eviction() {
        let mut repo = BestRepo::new(2);

        // Insert 3 records with different penalties
        for (i, penalty) in [(0, 10.0), (1, 8.0), (2, 12.0)].iter() {
            let mut layout = Layout::alphabetical();
            layout.swap(0, *i + 1); // Make different layouts

            let rec = ResultRecord {
                layout: layout_compact_string(&layout),
                mirrored: layout_compact_string(&mirror_layout(&layout)),
                canonical: canonical_key(&layout),
                penalty: *penalty,
                components: vec![],
                seed: *i as u64,
                worker_id: 0,
                iter: 100,
                ts: now_epoch(),
                corpus_fingerprint: "test".to_string(),
                config_fingerprint: "test".to_string(),
            };
            repo.insert(rec);
        }

        assert_eq!(repo.len(), 2); // Capacity is 2

        // Verify we kept the best two
        let top = repo.top();
        assert_eq!(top[0].penalty, 8.0);
        assert_eq!(top[1].penalty, 10.0);
        // Record with penalty 12.0 should have been evicted
    }

    #[test]
    fn test_deduplication_by_canonical_key() {
        let mut repo = BestRepo::new(10);

        let layout = Layout::alphabetical();
        let mirrored = mirror_layout(&layout);

        let rec1 = ResultRecord {
            layout: layout_compact_string(&layout),
            mirrored: layout_compact_string(&mirrored),
            canonical: canonical_key(&layout),
            penalty: 10.0,
            components: vec![],
            seed: 1,
            worker_id: 0,
            iter: 100,
            ts: now_epoch(),
            corpus_fingerprint: "test".to_string(),
            config_fingerprint: "test".to_string(),
        };

        let rec2 = ResultRecord {
            layout: layout_compact_string(&mirrored),
            mirrored: layout_compact_string(&layout),
            canonical: canonical_key(&mirrored),
            penalty: 9.0,
            components: vec![],
            seed: 2,
            worker_id: 1,
            iter: 200,
            ts: now_epoch(),
            corpus_fingerprint: "test".to_string(),
            config_fingerprint: "test".to_string(),
        };

        repo.insert(rec1);
        repo.insert(rec2);

        // Should only have one record since they have the same canonical key
        assert_eq!(repo.len(), 1);
        assert_eq!(repo.top()[0].penalty, 9.0); // Better penalty wins
    }
}
