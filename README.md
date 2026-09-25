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

## Keyboard

Digits, `+ - * / ^ ! % ( )`, `.`, `Enter`/`=`, `Backspace`, `Escape`, `p` (π), `e`.
