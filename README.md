# Rublock

A Rust learning project — a solver for a small grid puzzle.

## The Puzzle

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
