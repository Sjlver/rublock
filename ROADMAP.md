# Rublock roadmap

Rublock is a small puzzle solving project, to learn Rust. See README.md for the puzzle rules.

My ultimate goal is to count the number of valid 6x6 puzzles. To do so:

- create a Grid struct that contains a partially filled board. Grid Cells can be Empty, Number, or Black.
- generate all possible grids (see below for details)
- for each filled grid:
  - compute the targets for the filled grid
  - check to see if that is a valid puzzle, i.e., if it has exactly one solution
- report the number of grids, and the number that are valid puzzles

For the Grid enumeration, I'd like to learn Rust's multithreading support. I think we could breadth-first explore the tree to generate a work queue of suitable size, then pass each partially-filled grid (i.e., tree node) to a worker pool. IIRC Rust has a great library with par_iter().

For the solving; I'm guessing that the current propagate() method is not enough to solve all boards. Need to add a backtracking solver. It should be able to count solutions (and stop after a configurable max number -- 2 if we just want to test for solution uniqueness)

I'd like a progress bar too.

This enumeration part is probably it's own entry point, different from the current main function.

It might also be worth refactoring the code first, to move independent bits into their own files. For example, everything related to `propagate()` could go in a file of it's own. If that's idiomatic Rust.

---

Overall, I mainly want to learn Rust. So I'd like the code to be as clear as possible, and as idiomatic as possible. Feel free to correct me if I have wrong premises or bad habits from other programming languages. You can also add explanations for why things are done in a certain way, preferably at the function level or above.
