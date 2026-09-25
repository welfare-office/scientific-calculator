# Scientific Calculator

An Apple-style monochrome scientific calculator. The computation engine is
written in Rust and compiled to WebAssembly; the interface is React +
TypeScript + Vite.

## Stack

- `engine/` - Rust crate (`wasm-bindgen`), compiled with `wasm-pack --target web`
- `src/` - React 19 + TypeScript app (Vite), monochrome CSS design system
- `src/wasm/` - generated WASM/JS bindings (committed so `npm run dev` works out of the box)

## Commands

```bash
npm install            # frontend deps
npm run dev            # dev server
npm run build          # production build (tsc + vite)
npm run build:engine   # rebuild Rust engine -> src/wasm
npm run test:engine    # engine unit tests (cargo test)
```

Requires: Node 22+, Rust 1.9x+ with `wasm32-unknown-unknown` target, `wasm-pack`.

## Engine model

Expression-based evaluation (recursive-descent parser, real operator
precedence) with the iOS interaction model:

- live entry editing with grouping separators and Apple-style 9-digit display
- context-aware percent: `200 + 10%` evaluates to `220`
- repeat `=` re-applies the last binary operation
- nested parentheses, implicit multiplication (`2π`, `(2)(3)`)
- scientific functions: trig/hyperbolic (+ inverses via `2nd`), powers, roots,
  logs, factorial, `EE`, `Rand`, `π`, `e`
- `Deg`/`Rad` angle modes, memory keys (`mc m+ m- mr`)
- `C`/`AC`, backspace, `±`, error state with recovery

## Long computation mode

Operations that overflow `f64` are routed by size:

- `n!` ≤ 170 → instant `f64`; 171–1000 → inline arbitrary-precision result
  (shown as `d.eeeeeeeee…` + digit count, full text copyable via the copy button)
- `n!` ≤ 1,000,000 → **long mode**: a Web Worker drives a chunked job on
  `malachite` big integers (binary-counter merge keeps products balanced;
  `simd128` enabled for leaf products). Progress, cancel (AC/C or ✕), and
  automatic **checkpoint/resume** — state is written to OPFS (`calc-ckpt/`)
  ~twice a second, so a reload or cancel mid-run simply resumes when the
  job is re-entered. Finished results are also persisted to OPFS.
- `b^e` with integer `e` → same routing by result digit count
  (>280 digits inline bigint, >2,000 digits async, >8,000,000 refused as
  "Too large").

The main thread never blocks; the display shows live progress while keys
stay responsive for cancellation.

## Keyboard

Digits, `+ - * / ^ ! % ( )`, `.`, `Enter`/`=`, `Backspace`, `Escape`, `p` (π), `e`.
