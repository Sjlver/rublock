# Privacy

Rublock is a Doplo puzzle that runs entirely in your browser. The puzzle
solver is compiled to WebAssembly; nothing about the puzzle you are playing
leaves your device.

## What we store locally

Your in-progress puzzle, settings, dismissed hints, and a count of how many
puzzles you have solved are saved to `localStorage` so they survive a refresh
or a new browser tab. This data stays on your device; clearing your browser
storage erases it.

## Analytics

In production we count page views and a few anonymous events — for example,
how often the support card below is shown and clicked — with
[GoatCounter](https://www.goatcounter.com/), a privacy-respecting analytics
service that does not use cookies or fingerprinting and records aggregate
counts rather than per-user trails.

## Supporting the project

Rublock is free and shows no ads. Once in a while, after you solve a puzzle, a
small card invites you to support development on
[Liberapay](https://liberapay.com/) or [Ko-fi](https://ko-fi.com/). Following
one of those links takes you to that platform, whose own privacy policy then
applies. We don't embed their scripts or trackers — the card is just a link.

## Questions

Open an issue at <https://github.com/Sjlver/rublock>.
