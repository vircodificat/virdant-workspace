# wavir

Render a beautiful SVG waveform from a VCD (Value Change Dump) file.

```console
$ wavir trace.vcd          # -> trace.svg
```

Colors are drawn from the Virdant palette (brand green `#8FC225`, dark
surfaces, kind-dot accents).

## Features

- Whole-trace view: the full dump always fits the canvas, with pixel-level
  decimation (busy clocks render as shaded activity bands, not aliasing).
- Sensible time ticks: 1/2/5-based major ticks in human units (ps…s).
- Bus lanes with hex value annotations and X-transition markers.
- X (unknown) and Z (high-impedance) states render with distinct styles.
- `--theme dark|light|terminal` variants.
- `--range 1.2us..1.4ms` to zoom into a time window.
- `--signals` / `--exclude` glob filtering, `--group` hierarchical grouping.
- `--mark t1,t2` labeled time cursors.
- `--no-heat` hides the transition-density heat strip.
- `--html` emits an interactive viewer instead of a static SVG.
- `-o` chooses the output path.

## Build

```console
$ cargo build --release
```
