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

-----------------
Thu Apr  2 01:16:27 AM CEST 2026
2fa1f92 Implement the black arc consistency rule.
Work queue: 800 items (8 threads × 100 target).

Total valid grids:                 32448
Valid puzzles (unique soln):        7161
Time: 0.668 seconds (48544.5 grids per second)
Benchmark 1: ./target/release/enumerate
  Time (mean ± σ):     532.2 ms ±  37.9 ms    [User: 4170.8 ms, System: 10.4 ms]
  Range (min … max):   515.3 ms … 639.5 ms    10 runs
 
Hmm, this found four more solutions than the previous one.
I'm not sure what the issue is... guessing that the old version didn't fully
determine some of the blacks?

-----------------
Thu Apr  2 12:29:40 PM CEST 2026
1e9419e General arc consistency rule :)
Work queue: 800 items (8 threads × 100 target).

Total valid grids:                 32448
Valid puzzles (unique soln):        7161
Time: 1.144 seconds (28366.6 grids per second)
Benchmark 1: ./target/release/enumerate
  Time (mean ± σ):      1.231 s ±  0.054 s    [User: 9.456 s, System: 0.022 s]
  Range (min … max):    1.142 s …  1.295 s    10 runs
 
General arc consistency is actually slower. Bummer :( But the code is cooler :)

-----------------
Thu Apr  2 12:38:59 PM CEST 2026
e177812 Fixes from Claude review.
Work queue: 800 items (8 threads × 100 target).

Total valid grids:                 32448
Valid puzzles (unique soln):        7161
Time: 0.911 seconds (35599.2 grids per second)
Benchmark 1: ./target/release/enumerate
  Time (mean ± σ):      1.022 s ±  0.053 s    [User: 7.996 s, System: 0.027 s]
  Range (min … max):    0.945 s …  1.106 s    10 runs
 
Yay, Claude's review helped a bit to avoid temporary allocs.

-----------------
Sun Apr  5 03:54:55 PM CEST 2026
64ea131 Add per-row / per-column change tracking.
Work queue: 800 items (8 threads × 100 target).

Total valid grids:                 32448
Valid puzzles (unique soln):        7161
Time: 1.052 seconds (30847.2 grids per second)
Benchmark 1: ./target/release/enumerate
  Time (mean ± σ):     947.2 ms ±  16.7 ms    [User: 7443.7 ms, System: 9.5 ms]
  Range (min … max):   931.7 ms … 978.7 ms    10 runs
 
