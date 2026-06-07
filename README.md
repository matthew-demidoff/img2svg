# img2svg

img2svg is a browser-based vectorizer that turns a raster image into a layered
SVG while keeping its color detail. It runs entirely in your browser: the image
is processed locally and never uploaded to a server.

## The problem it targets

Most image-to-SVG converters either give you a black-and-white silhouette or a
mess of color regions that don't match the original. The usual reason is that
they trace the pixels as they find them, including JPEG compression artifacts
and anti-aliased edges, so a single visual color ends up split into dozens of
slightly different shades. The output is large, the colors drift, and the
result looks nothing like the source.

img2svg works the other way around. It decides what the real color regions are
*before* tracing, so the layers correspond to colors a person would actually
name in the image.

## How it works

The pipeline runs in a few stages:

1. **Perceptual quantization.** Colors are reduced in OKLab space, which is
   closer to how people perceive color difference than RGB. The reduced palette
   is then locked back to the colors that actually appear in the source, so the
   output doesn't introduce shades that weren't there.
2. **Stacked tracing.** Each color region is traced with
   [VTracer](https://github.com/visioncortex/vtracer) and emitted as its own
   layer.
3. **Overlap-then-cover layering.** Layers are stacked so that adjacent regions
   overlap slightly and upper layers cover the seams, which avoids the hairline
   gaps you get from edge-to-edge tiling.
4. **Gradient detection.** Smooth color transitions are detected and expressed
   as SVG gradients instead of many near-identical fills.
5. **Optimization.** The result is run through [SVGO](https://github.com/svg/svgo)
   to drop redundant data and shrink the file.

## Status

Early. This is an MVP. It is browser-only for now, and all processing happens
client-side through a WebAssembly build of the Rust core, so images stay on your
device. Expect rough edges and changing internals.

The OKLab quantization, source-palette locking, stacked color tracing, and SVGO
optimization are implemented today. Gradient detection and the one-pixel overlap
dilation described above are stubbed and not yet wired in, and the photographic
raster-embed fallback is defined but inactive.

## Building and running for development

For a one-command setup, run `./run.sh`. It installs the Rust toolchain, the
wasm32 target, wasm-pack, and the web dependencies, builds the WebAssembly core,
and starts the dev server. Use `./run.sh --build` to preview a production build
or `./run.sh --test` to run the Rust tests.

To do it by hand instead:

Prerequisites:

- Rust, installed via [rustup](https://rustup.rs/), with the
  `wasm32-unknown-unknown` target:
  ```
  rustup target add wasm32-unknown-unknown
  ```
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
- Node 20 or newer

Build the WebAssembly package from the core crate:

```
wasm-pack build crates/img2svg-core --target web --out-dir ../../web/src/wasm/pkg -- --features wasm
```

The `--features wasm` flag is required: the JavaScript bindings are gated behind
it, so a build without it produces a package with no exports.

Run the native test suite:

```
cargo test
```

Start the web dev server:

```
cd web
npm install
npm run dev
```

## License

Dual-licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option. Contributions are accepted under the same terms; see
[CONTRIBUTING.md](CONTRIBUTING.md).
