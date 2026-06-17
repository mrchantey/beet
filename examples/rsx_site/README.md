# rsx_site

A deliberately small **typed** beet site, demonstrating the Rust authoring path
that the no-code `examples/bsx_site` (and the canonical top-level `site/`) does
not exercise. It is the compiled counterpart to `beet serve site/`.

Three routes, exercising the interesting typed mechanisms:
- **`/`** — an `rsx!` hero page using `inline_class!` for one-off layout.
- **`/counter`** — native (typed) reactivity: a `Document` atom driven by
  `PointerUp` observers through a `FieldQuery`, the Rust twin of the no-code
  `bx:click` counter.
- **`/buttons`** — the `Button`/`Link` widgets laid out by a typed `Rule`
  (`design_row_rule`).

A trimmed typed `BeetLayout` composes the library `Header`/`Footer` around each
route body, and `server_plugin` is the shared render substrate.

## Run

One binary, three modes selected by build features (mirroring how the binary
serves http, runs a live TUI, or renders one route to stdout):

```sh
# 1. generate the typed routes (writes src/codegen/, git-ignored)
cargo run -p rsx_site --no-default-features --features codegen

# 2a. http server (default)
cargo run -p rsx_site

# 2b. live interactive TUI
cargo run -p rsx_site --features tui

# 2c. render one route to stdout and exit
cargo run -p rsx_site --features cli -- counter --accept=text/ansi-term
```

## Test

```sh
cargo test -p rsx_site            # render assertions
cargo test -p rsx_site --features tui   # + the live-TUI ChannelTerminal harness
```
