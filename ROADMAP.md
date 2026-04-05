# Rublock roadmap

Rublock is a small puzzle solving project, to learn Rust. See README.md for the puzzle rules and the goals of the project.

## Symmetry and enumeration

Puzzles have some inherent symmetry. We might benefit from that to speed up enumeration.

The easiest case is to make sure that row_targets is always lexicographically smaller or equal to col_targets. If if is smaller, we can count the number of solutions twice. If it is equal, once.

I guess there's a second symmetry: making sure that row_targets is smaller than/equal to its mirror. Same counting strategy as for the row/column pivot above.
