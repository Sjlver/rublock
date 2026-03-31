# Rublock roadmap

Rublock is a small puzzle solving project, to learn Rust. See README.md for the puzzle rules.

Currently, the solver in src/solver.rs is specialized for N=6. I want to make it general.

- Most of the size-specific code can be generalized if we generalize VALID_TUPLES first. We need one per size.
- The other constants like CANT_BE_INSIDE and D_MIN can be easily derived from VALID_TUPLES.

Overall, I'm not super sure it makes sense that SolverState has a `const N` template parameter. Maybe it does... after all, it would be nice if it is easily cloneable without heap allocations. But we have to find a good idiomatic way to make this work.

Next steps:
- generalize the solver for N >= 2
- Add some tests for other sizes. N=2 is an extreme case with just the all-black grid.
- change bin/enumerate.rs to enumerate puzzles of size 5 instead of 6. Size 6 takes a bit too long for iterative development.

After that:
- I'd also like to change the backtracking loop to do some nogood learning. If it does an assignment, and then the recursive call does not find any solution, then we know the assignment is invalid. We can remove it from the domain and recurse, rather than trying the other values first. The recursive call will call `propagate` and might make good use of the reduced domain.
