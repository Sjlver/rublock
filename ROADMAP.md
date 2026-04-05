# Rublock roadmap

Rublock is a small puzzle solving project, to learn Rust. See README.md for the puzzle rules and the goals of the project.

## Solver work queue or rule skipping

Currently, the solver runs the same rules over and over until a fixpoint is reached. This is a lot of unnecessary work. For example, the rules that clear BLACK bits run even if all the black cells are already determined.

I'd like to use something like a work queue instead (like in the AC-3 algorithm). If a cell's domain shrink, enqueue rules for that row and that column. Or generalize the `changed` boolean flag to a per-row and per-column flag. Rules that work on unchanged columns could be skipped. I kinda like this idea; it seems simpler than a work queue, and it plausibly reduces the amount of work that is done.

Maybe some rules could also have a cheap pre-conditions check at the start. For example, rules that clear BLACK bits can skip rows where the BLACK cells are already assigned.
