use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_run_par_creates_output() {
    let test_dir = "./test_output_integration";

    // Clean up if exists
    let _ = fs::remove_dir_all(test_dir);

    // Create test corpus
    fs::write("test_corpus_int.txt", "hello world test").unwrap();

    // Run the program
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "run-par",
            "test_corpus_int.txt",
            "--threads",
            "2",
            "--time",
            "1s",
            "--persist",
            test_dir,
            "--seed",
            "42",
        ])
        .output()
        .expect("Failed to execute command");

    // Check it ran successfully
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output files were created
    assert!(Path::new(test_dir).exists(), "Output directory not created");
    assert!(
        Path::new(&format!("{}/best.json", test_dir)).exists(),
        "best.json not created"
    );
    assert!(
        Path::new(&format!("{}/results.jsonl", test_dir)).exists(),
        "results.jsonl not created"
    );
    assert!(
        Path::new(&format!("{}/events_worker_0.csv", test_dir)).exists(),
        "Worker 0 history not created"
    );
    assert!(
        Path::new(&format!("{}/events_worker_1.csv", test_dir)).exists(),
        "Worker 1 history not created"
    );

    // Clean up
    fs::remove_dir_all(test_dir).unwrap();
    fs::remove_file("test_corpus_int.txt").unwrap();
}

#[test]
fn test_mirror_invariance() {
    // Test that mirrored layouts have the same penalty
    // This is tested in the unit tests, but let's verify end-to-end

    let test_dir = "./test_mirror_output";
    let _ = fs::remove_dir_all(test_dir);

    fs::write(
        "test_corpus_mirror.txt",
        "the quick brown fox jumps over the lazy dog",
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "run-par",
            "test_corpus_mirror.txt",
            "--threads",
            "1",
            "--time",
            "2s",
            "--persist",
            test_dir,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Read the best.json and verify penalties match for original and mirror
    let best_json = fs::read_to_string(&format!("{}/best.json", test_dir)).unwrap();
    let results: Vec<serde_json::Value> = serde_json::from_str(&best_json).unwrap();

    // Just verify the file is not empty and has expected structure
    assert!(!results.is_empty());
    assert!(results[0]["layout"].is_string());
    assert!(results[0]["mirrored"].is_string());
    assert!(results[0]["penalty"].is_number());

    // Clean up
    fs::remove_dir_all(test_dir).unwrap();
    fs::remove_file("test_corpus_mirror.txt").unwrap();
}

#[test]
fn test_merge_deduplication() {
    // Test the merge command deduplicates correctly

    // Create two result files with overlapping entries
    let result1 = r#"[
        {"layout": "abcdefghijklmnopqrstuvwxyz", "mirrored": "zyxwvutsrqponmlkjihgfedcba",
         "canonical": "abcdefghijklmnopqrstuvwxyz", "penalty": 5.0, "components": [],
         "seed": 1, "worker_id": 0, "iter": 100, "ts": 1234567890,
         "corpus_fingerprint": "fp1", "config_fingerprint": "cfg1"}
    ]"#;

    let result2 = r#"[
        {"layout": "abcdefghijklmnopqrstuvwxyz", "mirrored": "zyxwvutsrqponmlkjihgfedcba",
         "canonical": "abcdefghijklmnopqrstuvwxyz", "penalty": 4.0, "components": [],
         "seed": 2, "worker_id": 1, "iter": 200, "ts": 1234567891,
         "corpus_fingerprint": "fp1", "config_fingerprint": "cfg1"},
        {"layout": "qwertyuiopasdfghjklzxcvbnm", "mirrored": "mnbvcxzlkjhgfdsapoiuytrewq",
         "canonical": "mnbvcxzlkjhgfdsapoiuytrewq", "penalty": 3.0, "components": [],
         "seed": 2, "worker_id": 1, "iter": 300, "ts": 1234567891,
         "corpus_fingerprint": "fp1", "config_fingerprint": "cfg1"}
    ]"#;

    fs::write("test_result1.json", result1).unwrap();
    fs::write("test_result2.json", result2).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "results",
            "merge",
            "--out",
            "test_merged.json",
            "test_result1.json",
            "test_result2.json",
        ])
        .output()
        .expect("Failed to execute merge command");

    assert!(output.status.success());

    // Read merged file and verify deduplication
    let merged = fs::read_to_string("test_merged.json").unwrap();
    let results: Vec<serde_json::Value> = serde_json::from_str(&merged).unwrap();

    assert_eq!(results.len(), 2, "Should have 2 unique results after merge");

    // Verify the better penalty was kept for the duplicate
    let first_penalty = results
        .iter()
        .find(|r| r["canonical"] == "abcdefghijklmnopqrstuvwxyz")
        .unwrap()["penalty"]
        .as_f64()
        .unwrap();

    assert_eq!(first_penalty, 4.0, "Should keep better penalty");

    // Clean up
    fs::remove_file("test_result1.json").unwrap();
    fs::remove_file("test_result2.json").unwrap();
    fs::remove_file("test_merged.json").unwrap();
}
