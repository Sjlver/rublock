// Generates a printable booklet of 39 Doplo puzzles for a 39th-birthday gift.
//
// Output: a single self-contained HTML file (`booklet.html` by default) with
// 40 A5 pages — one title page plus one puzzle per page. Print it on 10
// sheets of A4 (two A5 pages per side, both sides) to fold into a booklet.
//
// Usage:
//   cargo run --release --bin gen_booklet
//   cargo run --release --bin gen_booklet -- --out=/tmp/booklet.html
//
// This is a one-off generator script intended to be run locally and printed.
// It is not wired into the web build.
//
// ── How it works ──────────────────────────────────────────────────────────────
//
// Difficulty buckets and their (search_nodes, propagation_waves) windows are
// the same as the web app's classifier in `web/src/state/classification.ts` —
// the thresholds there were already calibrated by hand, so we just mirror
// them. The matching German labels also come from `web/src/i18n/de.ts`.
//
// For each grid size, we race worker threads to fill empty random grids,
// derive their row/column targets, and check uniqueness with `Solver::solve`.
// Every uniquely-solvable puzzle is then classified into one of six
// difficulty buckets and stashed in the bucket's pool — but only if that
// bucket still needs more puzzles for the booklet. Workers exit as soon as
// every bucket the booklet wants is full.
//
// The HTML embeds all CSS inline and pulls a couple of Google Fonts via a
// stylesheet `<link>`; no other external assets, no web build step.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

use rublock::black_solver::BlackSolverState;
use rublock::grid::{Cell, random_grid};
use rublock::recorder::Recorder;
use rublock::solver::{Puzzle, SolveOutcome, Solver};

// ── Difficulty ────────────────────────────────────────────────────────────────

/// One of the six difficulty buckets the web app's classifier emits.
///
/// Order matters: the classifier picks the first bucket whose `(max_nodes,
/// max_waves)` envelope contains the puzzle's metrics, so the buckets here
/// are in strictly increasing difficulty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Difficulty {
    Easy,
    Medium,
    Challenging,
    Hard,
    VeryHard,
    ExtremelyHard,
}

impl Difficulty {
    /// German label, mirroring `web/src/i18n/de.ts`.
    fn label_de(self) -> &'static str {
        match self {
            Difficulty::Easy => "Leicht",
            Difficulty::Medium => "Mittel",
            Difficulty::Challenging => "Knifflig",
            Difficulty::Hard => "Schwer",
            Difficulty::VeryHard => "Sehr schwer",
            Difficulty::ExtremelyHard => "Extrem schwer",
        }
    }
}

/// `(max_nodes, max_waves)` per difficulty bucket, per supported size.
///
/// Mirrors `DIFFICULTY_THRESHOLDS` in `web/src/state/classification.ts`. The
/// classifier greedily picks the first bucket whose ceiling covers the
/// puzzle's metrics, so the implicit minimum for each bucket is one past the
/// previous bucket's ceiling on either axis.
fn thresholds(size: usize) -> &'static [(Difficulty, u64, u64)] {
    use Difficulty::*;
    const U: u64 = u64::MAX;
    match size {
        5 => &[
            (Easy, 1, 3),
            (Medium, 1, 6),
            (Challenging, 1, U),
            (Hard, U, 10),
            (VeryHard, U, 17),
            (ExtremelyHard, U, U),
        ],
        6 => &[
            (Easy, 1, 6),
            (Medium, 1, 10),
            (Challenging, 1, U),
            (Hard, U, 14),
            (VeryHard, U, 45),
            (ExtremelyHard, U, U),
        ],
        7 => &[
            (Easy, 1, 8),
            (Medium, 1, 15),
            (Challenging, 1, U),
            (Hard, U, 19),
            (VeryHard, U, 300),
            (ExtremelyHard, U, U),
        ],
        8 => &[
            (Easy, 1, 14),
            (Medium, 1, 22),
            (Challenging, 1, U),
            (Hard, U, 200),
            (VeryHard, U, 6000),
            (ExtremelyHard, U, U),
        ],
        _ => unreachable!("unsupported size {size}"),
    }
}

/// Classify a uniquely-solvable puzzle by its solver metrics.
///
/// Returns `None` if the puzzle's metrics fall outside every bucket, which
/// shouldn't happen — the last bucket's ceilings are `u64::MAX`.
fn classify(size: usize, nodes: u64, waves: u64) -> Option<Difficulty> {
    for &(d, max_n, max_w) in thresholds(size) {
        if nodes <= max_n && waves <= max_w {
            return Some(d);
        }
    }
    None
}

// ── Booklet spec ──────────────────────────────────────────────────────────────

/// One puzzle slot in the booklet.
#[derive(Clone, Copy, Debug)]
struct Spec {
    size: usize,
    difficulty: Difficulty,
}

/// The full ordered list of 39 puzzles. Grouped by size, easiest-first within
/// each size so the booklet ramps up gradually inside each section.
fn booklet_specs() -> Vec<Spec> {
    use Difficulty::*;
    let groups: [(usize, &[(Difficulty, usize)]); 4] = [
        (5, &[(Easy, 1), (Medium, 2), (Challenging, 2), (Hard, 1)]),
        (6, &[(Easy, 2), (Medium, 3), (Challenging, 7), (Hard, 1)]),
        (7, &[(Easy, 2), (Medium, 3), (Challenging, 7), (Hard, 1)]),
        (8, &[(Easy, 1), (Medium, 2), (Challenging, 3), (Hard, 1)]),
    ];

    let mut specs = Vec::new();
    for (size, buckets) in groups {
        for &(diff, count) in buckets {
            for _ in 0..count {
                specs.push(Spec {
                    size,
                    difficulty: diff,
                });
            }
        }
    }
    assert_eq!(specs.len(), 39);
    specs
}

// ── Pool entry / final puzzle ─────────────────────────────────────────────────

/// One uniquely-solvable puzzle, plus the metrics that determine its bucket.
/// Size-erased so heterogeneous sizes share the same code path.
#[derive(Clone)]
struct PoolEntry {
    size: usize,
    row_targets: Vec<u8>,
    col_targets: Vec<u8>,
    solved: Vec<Vec<Cell>>,
    nodes: u64,
    waves: u64,
}

/// A pool entry promoted into a final booklet slot with a difficulty label.
struct BookletPuzzle {
    size: usize,
    difficulty: Difficulty,
    row_targets: Vec<u8>,
    col_targets: Vec<u8>,
    #[allow(dead_code)]
    solved: Vec<Vec<Cell>>,
    nodes: u64,
    waves: u64,
}

// ── Pool collection ───────────────────────────────────────────────────────────

/// Buckets-of-puzzles state shared across worker threads. Workers append to
/// the per-difficulty `Vec`s; `still_needs` returns whether any required
/// bucket still has fewer puzzles than the booklet wants.
struct PoolBuckets {
    needs: HashMap<Difficulty, usize>,
    pools: HashMap<Difficulty, Vec<PoolEntry>>,
}

impl PoolBuckets {
    fn new(needs: HashMap<Difficulty, usize>) -> Self {
        let pools = needs.keys().map(|&d| (d, Vec::new())).collect();
        Self { needs, pools }
    }

    /// Append `entry` to its bucket if the bucket isn't full yet.
    /// Returns `true` if the entry was kept.
    fn try_add(&mut self, entry: PoolEntry, diff: Difficulty) -> bool {
        let Some(&need) = self.needs.get(&diff) else {
            return false;
        };
        let pool = self.pools.get_mut(&diff).unwrap();
        if pool.len() >= need {
            return false;
        }
        pool.push(entry);
        true
    }

    fn is_complete(&self) -> bool {
        self.needs
            .iter()
            .all(|(d, &n)| self.pools.get(d).map(|p| p.len()).unwrap_or(0) >= n)
    }
}

/// Collect puzzles of side length `N` until every requested bucket is full.
///
/// `needs` maps each requested difficulty to the number of puzzles the
/// booklet needs at that difficulty for size `N`. The function spawns
/// `threads` workers and returns once every bucket is full.
///
/// Workers race using two fast paths:
///   - For buckets requesting **propagation-only** puzzles (those whose
///     `max_nodes == 1`), workers run only `propagate()` and bail if the
///     state is not yet solved. This skips the much more expensive
///     backtracking search on grids that wouldn't have qualified anyway.
///   - For buckets that need backtracking (any `nodes >= 2` bucket), workers
///     fall back to the full `solve()` call.
///
/// The progress callback is called after every kept entry with the current
/// bucket counts, so callers can print a live status line.
fn collect_pool<const N: usize>(
    needs: HashMap<Difficulty, usize>,
    threads: usize,
    progress: impl Fn(&HashMap<Difficulty, usize>, &HashMap<Difficulty, usize>) + Send + Sync,
) -> Vec<PoolEntry> {
    let buckets = Mutex::new(PoolBuckets::new(needs));

    // Whether the puzzle for size N is a "backtracking bucket" — i.e. one
    // whose threshold accepts puzzles with `nodes >= 2`. Used to decide
    // whether the worker needs the expensive full solve, or can stop after
    // propagation.
    let is_backtracking_bucket =
        |d: &Difficulty| thresholds(N).iter().find(|(td, _, _)| td == d).unwrap().1 > 1;

    thread::scope(|s| {
        for _ in 0..threads {
            let buckets = &buckets;
            let progress = &progress;
            s.spawn(move || {
                let mut rng = rand::rng();
                loop {
                    // Re-check completion *and* whether any unfilled bucket
                    // still requires backtracking. Once all backtracking
                    // buckets are full, every remaining bucket only accepts
                    // propagation-solvable puzzles — so workers can drop the
                    // full solve and just propagate, which is dramatically
                    // faster on sizes where most random grids would need
                    // backtracking (size 8 especially).
                    let any_backtracking_unfilled = {
                        let b = buckets.lock().unwrap();
                        if b.is_complete() {
                            return;
                        }
                        b.needs.iter().any(|(d, &need)| {
                            is_backtracking_bucket(d)
                                && b.pools.get(d).map(|p| p.len()).unwrap_or(0) < need
                        })
                    };

                    let grid = random_grid::<N>(&mut rng);
                    let (row_targets, col_targets) = grid.compute_targets();
                    let puzzle = Puzzle::new(row_targets, col_targets);

                    // Try the propagation-only fast path first.
                    let mut prop_state = BlackSolverState::<N>::new(puzzle.clone());
                    prop_state.propagate();
                    let (nodes, waves, solved_grid) = if prop_state.is_solved() {
                        let waves = prop_state.recorder().propagation_waves();
                        let g = prop_state.solved_cells().expect("solved => determined");
                        (1u64, waves, g)
                    } else if !any_backtracking_unfilled {
                        // No backtracking bucket left to fill — and this
                        // grid isn't propagation-solvable — so it can't
                        // contribute. Skip the expensive full solve.
                        continue;
                    } else {
                        // Need a full solve to learn the node count.
                        let SolveOutcome::Unique(solved) =
                            BlackSolverState::<N>::new(puzzle).solve()
                        else {
                            continue;
                        };
                        let nodes = solved.recorder().search_nodes();
                        let waves = solved.recorder().propagation_waves();
                        let g = solved.solved_cells().expect("unique => determined");
                        (nodes, waves, g)
                    };

                    let Some(diff) = classify(N, nodes, waves) else {
                        continue;
                    };

                    let entry = PoolEntry {
                        size: N,
                        row_targets: row_targets.to_vec(),
                        col_targets: col_targets.to_vec(),
                        solved: solved_grid.cells.iter().map(|r| r.to_vec()).collect(),
                        nodes,
                        waves,
                    };
                    let added = {
                        let mut b = buckets.lock().unwrap();
                        b.try_add(entry, diff)
                    };
                    if added {
                        // Snapshot counts and invoke the progress callback
                        // outside the lock.
                        let counts: HashMap<Difficulty, usize> = {
                            let b = buckets.lock().unwrap();
                            b.pools.iter().map(|(d, p)| (*d, p.len())).collect()
                        };
                        let needs_snapshot: HashMap<Difficulty, usize> =
                            buckets.lock().unwrap().needs.clone();
                        progress(&needs_snapshot, &counts);
                    }
                }
            });
        }
    });

    let buckets = buckets.into_inner().unwrap();
    let mut out = Vec::new();
    for (_, mut pool) in buckets.pools {
        out.append(&mut pool);
    }
    out
}

fn collect_pool_dyn(
    size: usize,
    needs: HashMap<Difficulty, usize>,
    threads: usize,
    progress: impl Fn(&HashMap<Difficulty, usize>, &HashMap<Difficulty, usize>) + Send + Sync,
) -> Vec<PoolEntry> {
    match size {
        5 => collect_pool::<5>(needs, threads, progress),
        6 => collect_pool::<6>(needs, threads, progress),
        7 => collect_pool::<7>(needs, threads, progress),
        8 => collect_pool::<8>(needs, threads, progress),
        _ => unreachable!("unsupported size {size}"),
    }
}

// ── Citations ─────────────────────────────────────────────────────────────────

/// 39 citations, one per puzzle page. Each entry is (text, source).
const CITATIONS: [(&str, &str); 39] = [
    (
        "Durch Stolpern kommt man bisweilen weiter, man muss nur nicht fallen und liegenbleiben.",
        "Johann Wolfgang von Goethe",
    ),
    (
        "Ich möchte lieber alles verlieren und dich finden, Gott, als alles gewinnen und dich nicht finden.",
        "Augustin",
    ),
    (
        "Viele würden gern ein einfacheres Leben führen, wenn der Weg dorthin nicht so kompliziert wäre.",
        "Justus Jonas",
    ),
    (
        "Der ist kein freier Mensch, der sich nicht auch einmal dem Nichtstun hingeben kann.",
        "Cicero",
    ),
    (
        "Vielleicht gibt es schönere Zeiten, aber dies ist die unsere.",
        "Jean Paul Sartre",
    ),
    (
        "Wenn durch einen Menschen ein wenig mehr Liebe und Güte, ein wenig mehr Licht und Wahrheit in der Welt war, dann hat das Leben einen Sinn gehabt.",
        "Alfred Delp",
    ),
    (
        "Liebe den Herrn, deinen Gott, von ganzem Herzen, mit ganzem Willen, mit deiner ganzen Kraft und deinem ganzen Verstand! Und: Liebe deinen Nächsten wie dich selbst!",
        "Lukas 10, 27",
    ),
    (
        "Kümmere dich nicht um den Beifall von Leuten, die du nicht kennst oder die du verachtest.",
        "Leo Nikolajewitsch Tolstoj",
    ),
    (
        "Wenn alles wirklich so wäre, wie wir es wollten, würden die Leute sich beschweren, daß nichts mehr so ist, wie es einmal war.",
        "Pierre Dac",
    ),
    (
        "It is better to be a young June-bug than an old bird of paradise.",
        "Mark Twain",
    ),
    (
        "I know I have not found the answers to all of my questions. The answers I have found only serve to raise a whole set of new questions. In some ways I am as confused as ever, but I believe that I am confused on a higher level and about more important things.",
        "Earl C. Kelley (1951)",
    ),
    (
        "Nicht weil es schwer ist, wagen wir es nicht, sondern weil wir es nicht wagen, ist es schwer.",
        "Seneca, Epistulae Morales ad Lucilium, Brief CIV, 26",
    ),
    (
        "Courage is not the absence of fear, but the triumph over it.",
        "Nelson Mandela",
    ),
    (
        "Do what you can, with what you have, where you are.",
        "Theodore Roosevelt",
    ),
    (
        "Es ist nicht genug zu wissen, man muss auch anwenden. Es ist nicht genug zu wollen, man muss auch tun.",
        "Johann Wolfgang von Goethe",
    ),
    (
        "We are what we repeatedly do. Excellence, then, is not an act, but a habit.",
        "Will Durant",
    ),
    (
        "The greatest glory in living lies not in never falling, but in rising every time we fall.",
        "Nelson Mandela",
    ),
    (
        "Two roads diverged in a wood, and I — I took the one less traveled by, and that has made all the difference.",
        "Robert Frost, The Road Not Taken (1916)",
    ),
    (
        "Phantasie ist wichtiger als Wissen, denn Wissen ist begrenzt.",
        "Albert Einstein",
    ),
    (
        "Not all those who wander are lost.",
        "J.R.R. Tolkien, The Fellowship of the Ring (1954)",
    ),
    (
        "Happiness is not something ready-made. It comes from your own actions.",
        "Dalai Lama XIV",
    ),
    (
        "It's not what happens to you, but how you react to it that matters.",
        "Epiktet, Enchiridion",
    ),
    (
        "The unexamined life is not worth living.",
        "Sokrates (überliefert durch Platon, Apologie)",
    ),
    (
        "Be yourself; everyone else is already taken.",
        "Oscar Wilde",
    ),
    ("Try to be a rainbow in someone's cloud.", "Maya Angelou"),
    (
        "Spread love everywhere you go. Let no one ever come to you without leaving happier.",
        "Mutter Teresa",
    ),
    (
        "Wir leben alle unter dem gleichen Himmel, aber wir haben nicht alle denselben Horizont.",
        "Konrad Adenauer",
    ),
    (
        "Es sind die Begegnungen mit den Menschen, die das Leben lebenswert machen.",
        "Guy de Maupassant",
    ),
    (
        "Tell me, what is it you plan to do with your one wild and precious life?",
        "Mary Oliver, House of Light (1990)",
    ),
    (
        "The purpose of life is to live it, to taste experience to the utmost, to reach out eagerly and without fear for newer and richer experience.",
        "Eleanor Roosevelt, You Learn by Living (1960)",
    ),
    (
        "You have brains in your head. You have feet in your shoes. You can steer yourself any direction you choose.",
        "Dr. Seuss, Oh, the Places You'll Go! (1990)",
    ),
    (
        "It always seems impossible until it's done.",
        "Nelson Mandela",
    ),
    (
        "The future belongs to those who believe in the beauty of their dreams.",
        "Eleanor Roosevelt",
    ),
    (
        "You miss 100% of the shots you don't take.",
        "Wayne Gretzky",
    ),
    (
        "You can be the ripest, juiciest peach in the world, and there's still going to be somebody who hates peaches.",
        "Dita Von Teese",
    ),
    (
        "In the depth of winter, I finally learned that within me there lay an invincible summer.",
        "Albert Camus, Retour à Tipasa, in: L'Été (1954)",
    ),
    (
        "Ich möchte Sie bitten, lieber Herr, Geduld zu haben gegen alles Ungelöste in Ihrem Herzen und zu versuchen, die Fragen selbst lieb zu haben.",
        "Rainer Maria Rilke, Briefe an einen jungen Dichter (1903)",
    ),
    (
        "For all that has been — Thanks! To all that shall be — Yes!",
        "Dag Hammarskjöld, Markings (Vägmärken, 1963)",
    ),
    (
        "Despite everything, life is full of beauty and meaning.",
        "Etty Hillesum, Tagebücher (1941–1943)",
    ),
];

// ── HTML rendering ────────────────────────────────────────────────────────────

const BOOKLET_CSS: &str = r#"
@page {
  size: A5 portrait;
  margin: 0;
}

* { box-sizing: border-box; }

html, body {
  margin: 0;
  padding: 0;
  background: #e8e6e1;
  color: #1a1a1a;
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.page {
  width: 148mm;
  height: 210mm;
  padding: 14mm 14mm;
  margin: 10mm auto;
  background: #fffdf8;
  page-break-after: always;
  break-after: page;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  position: relative;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.page:last-of-type {
  page-break-after: auto;
  break-after: auto;
}

@media print {
  html, body { background: #fff; }
  .page { margin: 0; box-shadow: none; background: #fff; }
}

/* ── Title page ── */

.title-page {
  text-align: center;
  justify-content: center;
}

.title-page .pretitle {
  font-family: 'Inter', sans-serif;
  font-size: 11pt;
  font-weight: 500;
  letter-spacing: 0.32em;
  text-transform: uppercase;
  color: #8a7d63;
  margin-bottom: 22mm;
}

.title-page h1 {
  font-family: 'Playfair Display', 'Georgia', serif;
  font-size: 38pt;
  font-weight: 700;
  font-style: italic;
  line-height: 1.15;
  margin: 0 0 12mm 0;
  color: #2a2418;
  letter-spacing: -0.01em;
}

.title-page h1 .num {
  font-style: normal;
  color: #8b1538;
}

.title-page .ornament {
  font-family: 'Playfair Display', serif;
  font-size: 18pt;
  letter-spacing: 0.6em;
  margin: 8mm 0 12mm 0;
  color: #b4a37f;
}

.title-page .subtitle {
  font-family: 'Playfair Display', serif;
  font-size: 16pt;
  font-style: italic;
  color: #4a3f2a;
  margin: 0;
}

/* ── Puzzle page ── */

.puzzle-page { justify-content: space-between; }

.puzzle-meta {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-family: 'Inter', sans-serif;
  font-size: 10pt;
  font-weight: 500;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #8a7d63;
}

.puzzle-meta .num {
  color: #2a2418;
  font-weight: 600;
}

.puzzle-meta .difficulty {
  color: #8b1538;
  font-weight: 600;
}

.puzzle-stage {
  flex: 1;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.puzzle-footer {
  width: 100%;
  border-top: 0.3pt solid #c9bda0;
  padding-top: 2.5mm;
  text-align: center;
}

.citation-text {
  font-family: 'Playfair Display', serif;
  font-style: italic;
  font-size: 8pt;
  color: #2a2418;
  line-height: 1.5;
  margin-bottom: 1.5mm;
}

.citation-source {
  font-family: 'Inter', sans-serif;
  font-style: italic;
  font-size: 7.5pt;
  color: #8a7d63;
  letter-spacing: 0.02em;
}

/* ── Grid ── */

table.puzzle {
  border-collapse: collapse;
  font-family: 'JetBrains Mono', ui-monospace, monospace;
  background: #fffdf8;
}

table.puzzle th,
table.puzzle td {
  border: 1px solid #2a2418;
  text-align: center;
  vertical-align: middle;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  color: #1a1a1a;
}

/* Cell size adapts to grid size so larger grids still fit comfortably on A5. */
table.puzzle.size-5 th, table.puzzle.size-5 td {
  width: 17mm; height: 17mm; font-size: 16pt;
}
table.puzzle.size-6 th, table.puzzle.size-6 td {
  width: 15mm; height: 15mm; font-size: 14pt;
}
table.puzzle.size-7 th, table.puzzle.size-7 td {
  width: 13mm; height: 13mm; font-size: 12pt;
}
table.puzzle.size-8 th, table.puzzle.size-8 td {
  width: 12mm; height: 12mm; font-size: 11pt;
}

table.puzzle th.target {
  background: #f0ebe0;
  color: #2a2418;
  font-weight: 600;
}

table.puzzle th.corner {
  border: none;
  background: transparent;
}

table.puzzle td.cell { background: #fffdf8; }

table.puzzle td.cell.black {
  background: #2a2418;
  print-color-adjust: exact;
  -webkit-print-color-adjust: exact;
}
"#;

fn render_grid_html(
    size: usize,
    row_targets: &[u8],
    col_targets: &[u8],
    solution: Option<&[Vec<Cell>]>,
) -> String {
    let mut s = String::new();
    s.push_str(&format!("<table class=\"puzzle size-{size}\">\n"));

    s.push_str("  <thead><tr><th class=\"corner\"></th>");
    for &t in col_targets {
        s.push_str(&format!("<th class=\"target\">{t}</th>"));
    }
    s.push_str("</tr></thead>\n");

    s.push_str("  <tbody>\n");
    for (r, &row_t) in row_targets.iter().enumerate() {
        s.push_str("    <tr>");
        s.push_str(&format!("<th class=\"target\">{row_t}</th>"));
        for c in 0..size {
            match solution.map(|g| g[r][c]) {
                Some(Cell::Black) => s.push_str("<td class=\"cell black\"></td>"),
                Some(Cell::Number(n)) => s.push_str(&format!("<td class=\"cell\">{n}</td>")),
                Some(Cell::Empty) | None => s.push_str("<td class=\"cell\"></td>"),
            }
        }
        s.push_str("</tr>\n");
    }
    s.push_str("  </tbody>\n");
    s.push_str("</table>\n");
    s
}

fn render_title_page() -> String {
    String::from(
        r#"
<section class="page title-page">
  <div class="pretitle">Doplo</div>
  <h1><span class="num">39</span> R&auml;tsel<br>zum <span class="num">39.</span>&nbsp;Geburtstag</h1>
  <div class="ornament">&#10070; &#10070; &#10070;</div>
  <div class="subtitle">Viel Spa&szlig; beim Kn&ouml;beln!</div>
</section>
"#,
    )
}

fn render_puzzle_page(
    index: usize,
    total: usize,
    puzzle: &BookletPuzzle,
    citation: (&str, &str),
) -> String {
    let grid_html = render_grid_html(puzzle.size, &puzzle.row_targets, &puzzle.col_targets, None);
    let (cite_text, cite_source) = citation;
    format!(
        r#"
<section class="page puzzle-page">
  <header class="puzzle-meta">
    <span class="num">Nr.&nbsp;{index}&thinsp;/&thinsp;{total}</span>
    <span class="difficulty">{diff}&nbsp;&middot;&nbsp;{size}&times;{size}</span>
  </header>
  <div class="puzzle-stage">
    {grid_html}
  </div>
  <footer class="puzzle-footer">
    <div class="citation-text">{cite_text}</div>
    <div class="citation-source">&mdash; <em>{cite_source}</em></div>
  </footer>
</section>
"#,
        index = index,
        total = total,
        diff = puzzle.difficulty.label_de(),
        size = puzzle.size,
        grid_html = grid_html,
        cite_text = cite_text,
        cite_source = cite_source,
    )
}

fn render_booklet_html(puzzles: &[BookletPuzzle]) -> String {
    let mut out = String::new();
    out.push_str(
        r#"<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<title>39 R&auml;tsel zum 39. Geburtstag</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@500;600&family=Playfair+Display:ital,wght@0,600;0,700;1,400;1,600;1,700&display=swap">
<style>
"#,
    );
    out.push_str(BOOKLET_CSS);
    out.push_str("\n</style>\n</head>\n<body>\n");
    out.push_str(&render_title_page());
    let total = puzzles.len();
    for (i, p) in puzzles.iter().enumerate() {
        out.push_str(&render_puzzle_page(i + 1, total, p, CITATIONS[i]));
    }
    out.push_str("</body>\n</html>\n");
    out
}

// ── Args ──────────────────────────────────────────────────────────────────────

struct Args {
    out: PathBuf,
    threads: usize,
}

fn parse_args() -> Args {
    let mut out = PathBuf::from("booklet.html");
    let mut threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("--out=") {
            out = PathBuf::from(v);
        } else if let Some(v) = arg.strip_prefix("--threads=") {
            threads = v.parse().unwrap_or_else(|_| {
                eprintln!("--threads must be a positive integer");
                std::process::exit(1);
            });
        } else {
            eprintln!("Usage: gen_booklet [--out=PATH] [--threads=N]");
            std::process::exit(1);
        }
    }
    Args { out, threads }
}

// ── Main ──────────────────────────────────────────────────────────────────────

/// Aggregate the booklet spec into per-size demand: for each size, how many
/// puzzles of each difficulty does the booklet need?
fn needs_per_size(specs: &[Spec]) -> HashMap<usize, HashMap<Difficulty, usize>> {
    let mut out: HashMap<usize, HashMap<Difficulty, usize>> = HashMap::new();
    for sp in specs {
        *out.entry(sp.size)
            .or_default()
            .entry(sp.difficulty)
            .or_default() += 1;
    }
    out
}

fn main() {
    let args = parse_args();
    let specs = booklet_specs();
    println!(
        "Generating {} puzzles using {} threads.",
        specs.len(),
        args.threads
    );

    let start = Instant::now();
    let demand = needs_per_size(&specs);

    // Collect pools per size, ordered by size so the log reads naturally.
    let mut sizes: Vec<usize> = demand.keys().copied().collect();
    sizes.sort();

    // For each size, hold its uniquely-solvable puzzles grouped by difficulty
    // bucket so we can hand them out to slots in spec order.
    let mut by_size: HashMap<usize, HashMap<Difficulty, Vec<PoolEntry>>> = HashMap::new();
    for &size in &sizes {
        let needs = demand[&size].clone();
        let t0 = Instant::now();
        let needs_summary: String = {
            let mut pairs: Vec<(Difficulty, usize)> = needs.iter().map(|(&d, &n)| (d, n)).collect();
            pairs.sort_by_key(|(d, _)| difficulty_order(*d));
            pairs
                .iter()
                .map(|(d, n)| format!("{}={}", d.label_de(), n))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("Size {size}: need {needs_summary}");
        let pool = collect_pool_dyn(size, needs, args.threads, |needs, counts| {
            // One line per kept entry. Stays on one line via \r so progress
            // doesn't flood the log.
            let mut diffs: Vec<Difficulty> = needs.keys().copied().collect();
            diffs.sort_by_key(|d| difficulty_order(*d));
            let parts: Vec<String> = diffs
                .iter()
                .map(|d| {
                    format!(
                        "{}: {}/{}",
                        d.label_de(),
                        counts.get(d).copied().unwrap_or(0),
                        needs.get(d).copied().unwrap_or(0),
                    )
                })
                .collect();
            print!("  \r  {}", parts.join("   "));
            std::io::Write::flush(&mut std::io::stdout()).ok();
        });
        let dt = t0.elapsed().as_secs_f64();
        // Clear the in-place progress line and print the final summary.
        println!("\n  -> collected {} in {dt:.1}s", pool.len());

        let mut grouped: HashMap<Difficulty, Vec<PoolEntry>> = HashMap::new();
        for entry in pool {
            // Re-classify: the bucket the pool was added to is the one we
            // want, but we deliberately re-derive it from the metrics so the
            // grouping stays canonical even if `try_add` logic changes.
            let diff = classify(size, entry.nodes, entry.waves)
                .expect("classify covers every metric combo");
            grouped.entry(diff).or_default().push(entry);
        }
        // Sort each bucket by waves so successive booklet slots in a bucket
        // get a gentle progression rather than a random shuffle.
        for v in grouped.values_mut() {
            v.sort_by_key(|e| (e.nodes, e.waves));
        }
        by_size.insert(size, grouped);
    }

    // Fill booklet slots in spec order.
    let mut puzzles: Vec<BookletPuzzle> = Vec::with_capacity(specs.len());
    for spec in &specs {
        let entry = by_size
            .get_mut(&spec.size)
            .and_then(|m| m.get_mut(&spec.difficulty))
            .and_then(|v| v.pop())
            .unwrap_or_else(|| {
                panic!(
                    "no puzzle available for size {} difficulty {:?}",
                    spec.size, spec.difficulty
                )
            });
        puzzles.push(BookletPuzzle {
            size: entry.size,
            difficulty: spec.difficulty,
            row_targets: entry.row_targets,
            col_targets: entry.col_targets,
            solved: entry.solved,
            nodes: entry.nodes,
            waves: entry.waves,
        });
    }

    println!();
    println!("Booklet contents:");
    for (i, p) in puzzles.iter().enumerate() {
        println!(
            "  {:>2}. size {} {:<14} nodes={:<5} waves={}",
            i + 1,
            p.size,
            p.difficulty.label_de(),
            p.nodes,
            p.waves,
        );
    }

    let total = start.elapsed().as_secs_f64();
    println!("Total generation time: {total:.1}s");

    let html = render_booklet_html(&puzzles);
    fs::write(&args.out, html).expect("failed to write booklet HTML");
    println!("Wrote {}", args.out.display());
}

/// Sort key so the per-size needs summary prints easiest-first.
fn difficulty_order(d: Difficulty) -> u8 {
    match d {
        Difficulty::Easy => 0,
        Difficulty::Medium => 1,
        Difficulty::Challenging => 2,
        Difficulty::Hard => 3,
        Difficulty::VeryHard => 4,
        Difficulty::ExtremelyHard => 5,
    }
}
