# Rublock

A Rust learning project — a solver for a small grid puzzle.

## The Puzzle

The puzzle is called **Doplo** and is published by *Küng Rätsel* at <https://doplo.ch>. I didn't invent it — I just wrote this solver (with a lot of assistance from Anthropic's Claude).

The puzzle is played on a **6x6 grid**. Each row and each column has a **target number** attached, ranging from 0 to 10.

### Rules

1. Each row and each column must contain exactly **two black squares**.
2. The remaining four squares in each row and column must contain the numbers **1, 2, 3, and 4** (each exactly once — a permutation).
3. The **sum of the numbers between the two black squares** in each row and column must equal that row/column's target number.

### Example

If a row has a target of **5** and its black squares are at columns 2 and 5 (1-indexed), then the numbers at columns 3 and 4 must sum to 5 — for example, 1 and 4, or 2 and 3.

If the two black squares are **adjacent**, there are no numbers between them, and the sum is 0.

## Goal

Write a solver in Rust that, given the 12 target numbers (6 for rows, 6 for columns), finds a valid assignment of black squares and digits to the grid.

## Project Structure

This project is primarily a vehicle for learning Rust — exploring ownership, iterators, enums, pattern matching, and more.

Because of that, I prefer code that is simple over code that maximizes performance at all costs. I also want code that is highly idiomatic and corresponds to best practices.

### Binaries

There are multiple solvers, accessible via `src/main.rs`.

There is a generator for puzzles of varying difficulty, in `src/bin/gen_puzzle.rs`. Here's the hardest puzzle it has found so far:

```
0 4 3 8 4 3 0 0 0 0 4 0

     0   0   0   0   4   0
   +---+---+---+---+---+---+
 0 | 3 | 1 | 2 | 4 | # | # |
   +---+---+---+---+---+---+
 4 | 4 | 2 | # | 1 | 3 | # |
   +---+---+---+---+---+---+
 3 | # | 3 | # | 2 | 1 | 4 |
   +---+---+---+---+---+---+
 8 | # | 4 | 1 | 3 | # | 2 |
   +---+---+---+---+---+---+
 4 | 1 | # | 4 | # | 2 | 3 |
   +---+---+---+---+---+---+
 3 | 2 | # | 3 | # | 4 | 1 |
   +---+---+---+---+---+---+

search nodes: 323  (after 3119192 grids)
```

There's also a binary to count the number of valid puzzles of a given size. It works well up to size 6, but sizes larger than that become prohibitively expensive.

One additional binary, `compare`, exists for development: it runs both solver backends on a fixed set of puzzles and asserts they agree.
