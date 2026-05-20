# Privacy

Rublock is a Doplo puzzle that runs entirely in your browser. The
puzzle solver is compiled to WebAssembly; nothing about the puzzle you
are playing leaves your device.

## What we store locally

Your in-progress puzzle, settings, and dismissed hints are saved to
`localStorage` so the puzzle survives a refresh or a new browser tab.
This data is on your device only; clearing your browser storage erases
it.

## Analytics

In production we count page views with
[GoatCounter](https://www.goatcounter.com/), a privacy-respecting
analytics service that does not use cookies or fingerprinting and
records aggregate counts rather than per-user trails.

## Ads

After you solve a puzzle we show a single ad served by
[EthicalAds](https://www.ethicalads.io/). EthicalAds does not use
cookies, does not fingerprint your device, and only records aggregate
impression counts. Their
[privacy policy](https://www.ethicalads.io/privacy-policy/) has the
full details.

## Questions

Open an issue at <https://github.com/Sjlver/rublock>.
