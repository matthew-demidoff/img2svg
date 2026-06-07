# Contributing to img2svg

Thanks for your interest in the project. It is early and the internals move
around, so opening an issue to discuss a larger change before you write it is
usually a good idea.

## Setting up the toolchain

You need:

- Rust via [rustup](https://rustup.rs/) with the `wasm32-unknown-unknown`
  target:
  ```
  rustup target add wasm32-unknown-unknown
  ```
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
- Node 20 or newer

## Checks before you push

Run the same checks CI runs, so you find problems locally first.

Rust:

```
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

WebAssembly build:

```
wasm-pack build crates/img2svg-core --target web --out-dir ../../web/src/wasm/pkg -- --features wasm
```

Web app:

```
cd web
npm install
npm run build
```

## Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/). Use the
imperative mood ("add gradient detector", not "added" or "adds"). Keep the
subject line plain ASCII. No emoji and no co-author trailers.

Examples:

```
feat: detect linear gradients in quantized regions
fix: avoid hairline seams between adjacent layers
docs: explain the OKLab quantization step
```

## Pull requests

Keep a PR to one concern. A focused change is easier to review and easier to
revert if it turns out to be wrong. If you find yourself fixing two unrelated
things, split them.

CI must be green before a PR can merge. If CI fails, the change is not done yet.
