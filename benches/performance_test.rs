// Simple performance test to verify multi-threading scales
// Run with: cargo run --release -- run-par corpus.txt --threads N --time 10s

use std::process::Command;
use std::time::Instant;

fn main() {
    // This is not a real benchmark harness, just a simple test
    // to verify that multi-threading improves performance

    println!("Performance scaling test for keygen");
    println!("===================================");

    // Create a reasonable test corpus
    let corpus = "the quick brown fox jumps over the lazy dog. ".repeat(100);
    std::fs::write("bench_corpus.txt", corpus).unwrap();

    for threads in [1, 2, 4] {
        println!("\nTesting with {} thread(s)...", threads);

        let start = Instant::now();

        let output = Command::new("cargo")
            .args([
                "run",
                "--release",
                "--",
                "run-par",
                "bench_corpus.txt",
                "--threads",
                &threads.to_string(),
                "--time",
                "5s",
                "--persist",
                &format!("./bench_output_{}", threads),
                "--seed",
                "42",
            ])
            .output()
            .expect("Failed to run command");

        let duration = start.elapsed();

        if output.status.success() {
            // Check the output for the final repository size
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout
                .lines()
                .find(|l| l.contains("Final repository size:"))
            {
                println!("  {}", line);
            }
            println!("  Total time: {:.2}s", duration.as_secs_f32());
        } else {
            println!("  ERROR: Command failed");
            println!("  {}", String::from_utf8_lossy(&output.stderr));
        }

        // Clean up
        let _ = std::fs::remove_dir_all(format!("./bench_output_{}", threads));
    }

    // Clean up corpus
    std::fs::remove_file("bench_corpus.txt").unwrap();

    println!("\nNote: This is a simple test to verify multi-threading works.");
    println!("For real benchmarks, iterations found should increase with more threads.");
}
