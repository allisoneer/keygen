# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**keygen** is a keyboard layout optimization tool that uses simulated annealing to generate ergonomic 26-letter keyboard layouts by minimizing physical discomfort. Inspired by [Carpalx](http://mkweb.bcgsc.ca/carpalx/?simulated_annealing) but with a different penalty model focusing on comfort maximization.

## Common Commands

### Building and Testing
```bash
# Development build (already optimized with opt-level=3)
cargo build

# Release build for performance testing
cargo build --release

# Run all tests
cargo test

# Run tests with output visible
cargo test -- --nocapture

# Run integration test
cargo test --test integration_test

# Run specific module tests
cargo test corpus::

# Code quality
cargo fmt
cargo clippy --all-targets
```

### Using the Optimizer
```bash
# Single-threaded optimization
cargo run -- run corpus/books.short.txt

# Parallel optimization (recommended)
cargo run --release -- run-par corpus/books.short.txt --threads 8 --time 30s

# With persistence (saves progress, resumable)
cargo run --release -- run-par corpus.txt --threads 8 --persist ./results --seed 12345

# Refine an existing layout
cargo run -- refine "abcdefghijklmnopqrstuvwxyz" corpus.txt

# View results
cargo run -- results show --file ./results/best.json --top 5
```

### Makefile Targets
```bash
make check      # Format + clippy + build
make test       # Run all tests
make build      # Release build
make all        # Full CI workflow: check → test → build
```

## Architecture Overview

### Core Data Flow

```
Corpus → Normalize → Extract n-grams → Score layouts → Anneal → Results
```

**Three execution modes:**
1. **run**: Single-threaded annealing from random start
2. **run-par**: Multi-threaded parallel annealing with result aggregation
3. **refine**: Local hill-climbing optimization from a given layout
