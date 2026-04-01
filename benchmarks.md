# Benchmarks for rublock

6x6 grids enumeration, with prefix "0 1 2 3 4 0 / 1 2"

```bash
./benchmark.sh >> benchmarks.md
```

-----------------
Wed Apr  1 04:40:18 PM CEST 2026
4e3bf8a Add configurable start state to enumerate.rs.
Work queue: 800 items (8 threads × 100 target).

Total valid grids:          32448
Valid puzzles (unique soln): 7157
Benchmark 1: ./target/release/enumerate
  Time (mean ± σ):     893.5 ms ±  24.4 ms    [User: 6868.3 ms, System: 6.1 ms]
  Range (min … max):   855.0 ms … 927.7 ms    10 runs
