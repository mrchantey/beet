# Run Examples

Use the workspace examples as smoke tests to catch regressions that unit and integration tests may have missed.

## Context

Not every example is verifiable from the CLI — many are 3D/2D Bevy windowed apps, browser-driven, or TUIs that block on stdin. This skill only exercises the ones that exit on their own (or that we can probe with `curl` while they run in the background).

Stream the entire output of each example into `.agents/tmp/scratch.txt` (overwrite for the first command, append for subsequent ones), then grep that file. This avoids reruns when checking multiple things.

Treat compile failures the same as `run-tests.md`: retry once, and if a mold linker error persists (`RUST_MIN_STACK`, "section sizes" etc) bump the workspace `version = "0.0.9-dev.N"` in `Cargo.toml`.

Long-running examples must always be wrapped in `timeout` (default 60s — drop it lower once you know the example exits faster). Server examples should be launched with `run_in_background` and killed once the probe has succeeded.

## Smoke Set

The set below was chosen so that each feature gate combination is exercised by at least one example, and each crate has at least one self-terminating verifier. Most examples just need an `OK` exit status; a few have specific output to grep for (noted inline).

### 1. Action (`--features=action`)

Covers the action runtime end-to-end: pure handlers, async handlers, control-flow nodes, state machines, score-based selectors, and timers.

```sh
cargo run --example hello_world     --features=action       # prints "Hello, world!"
cargo run --example simple_action   --features=action       # caller-entity lookup
cargo run --example behavior_tree   --features=action       # sequence + log
cargo run --example state_machine   --features=action       # RunNext jumps
cargo run --example repeat_while    --features=action       # loop + condition
cargo run --example utility_ai      --features=action       # HighestScore
cargo run --example long_running    --features=action       # 1.3s timer chain
cargo run --example malenia         --features=action,rand  # BT + utility AI
```

### 2. Scripting (`--features=rhai_serde`)

```sh
cargo run --example scripting --features=rhai_serde         # rhai Script<I,O>
```

### 3. Router (`--features=router,markdown` / plus extras)

CLI router server, persisted router, and the codegen pipeline.

```sh
cargo run --example router           --features=router,markdown
cargo run --example router           --features=router,markdown -- about
cargo run --example cli              --features=router,rhai_serde -- greet --name=world
cargo run --example router_serde     --features=router,rhai_serde,template_serde
cargo run --example router_serde     --features=router,rhai_serde,template_serde -- greet --name=world
# rsx_site is a crate, not a root example: generate its routes, then serve. It
# scans typed pages, markdown content and a server action from three collections.
cargo run -p rsx_site --no-default-features --features codegen   # regenerate src/codegen/
cargo run -p rsx_site                                            # http server (default)
cargo run -p rsx_site --features cli -- guide --accept=text/html # render one route to stdout
```

### 4. Todo (`--features=router,json`)

Round-trip the todo document: list → create → list → delete → list.

```sh
cargo run --example todo --features=router,json -- list
cargo run --example todo --features=router,json -- create --body='{"description":"smoke test","done":false}'
cargo run --example todo --features=router,json -- list
cargo run --example todo --features=router,json -- delete --body=0
```

### 5. Net (`--features=net,ureq,native-tls` / `--features=http_server`)

`http_client` hits `example.com` and asserts on the response body — skip if offline.

```sh
cargo run --example http_client --features=net,ureq,native-tls
```

For the server side, run in background and probe with `curl`:

```sh
# launch
cargo run --example http_server --features=http_server     # background
curl -s http://localhost:8337                              # expect 200 + body
curl -s http://localhost:8337?name=billy
# kill the background pid
```

Same pattern for `templating` (`--features=http_server`) and `style` (`--features=http_server,style`).

### 6. Per-crate examples

These belong to a specific crate so they need `-p`.

```sh
cargo run -p beet_core --example runner                    # custom test runner
cargo run -p beet_core --example tracing                   # PrettyTracing init
cargo run -p beet_ui   --example render_simple             # oneshot terminal render
cargo run -p beet_ui   --example inline_formatting         # block + inline runs
cargo run -p beet_ui   --example reactive                  # prints "success"
cargo run -p beet_ui   --example build_css   --features=style       # writes target/examples/style/*
cargo run -p beet_ml   --example hello_ml_basic                     # downloads bert (~90MB, slow first run)
cargo run -p beet_ml   --example hello_rl_basic --features=bevy_default
```

### 7. Workspace ML (`--features=examples,ml`)

The `examples,ml` feature only gates windowed scene code (now scene modules in
`beet_examples`, not runnable `--example` targets), so there is no
self-terminating CLI smoke here. The runtime ML smoke lives in the crate
(`hello_ml_basic`, section 6); this feature's compilation is covered by the
skip-set check below (and is the only coverage, since `beet_examples` is
excluded from the test crates).

### 8. BSX scenes (`just beet --main=<file>.bsx`)

The no-code `.bsx` scenes run through the beet CLI (`just beet` = `cargo run -p
beet-cli --features render,ml`). The self-terminating ones render and exit:

```sh
just beet --main=examples/hello/main.bsx       # prints "hello world"
just beet --main=examples/ml/hello_ml.bsx      # logs "NearestSentence chose: ..."
```

Skip: `examples/spatial/*.bsx` and `examples/ml/frozen_lake_*.bsx` (windowed),
`examples/thread/*.bsx` (need an LLM key), `examples/bsx_site/main.bsx` (HTTP
server). `examples/calculator/main.bsx` currently SIGSEGVs on process teardown
under the render+ml backend — flagged for investigation.

## Not Verifiable Via CLI (skip)

Documented so future passes don't waste time on them:

- **Spatial / ML scenes:** `flock`, `seek`, `fetch`, `frozen_lake_run` etc. are no longer `--example` targets — they live as scene modules in `beet_examples`, reached only by building the `examples,spatial` / `examples,ml` features.
- **Thread scenes:** `chat`, `multi_agent`, `oneshot`, `persistent_chat`, `tool_call`, `self_evolving`, `coding_agent` are `.bsx` markup scenes under `examples/thread/`, not `--example` targets. Several also need an LLM key (`OPENAI_API_KEY` / `BEDROCK_*`).
- **Interactive TUI/stdin:** `ui/term_input`, `ui/tui`, `ui/state` — real examples that block on stdin.
- **Browser required:** `ui/crud`, `ui/syntax_highlighting`, `ui/media_renderer` (interactive output).
- **Needs AWS:** `hello_lambda`, `hello_lightsail`, `hello_fargate`, `lifecycle`. Compile-check only.
- **Needs sshd:** `ssh_server`, `ssh_client`, `ssh_tui`.

A pure compile check is still useful for the skipped set. The first three cover
`beet_examples` and the feature gates, which no test crate compiles:

```sh
cargo check -p beet --features=examples,ml       # beet_examples ML scenes (fetch etc.)
cargo check -p beet --features=examples,spatial  # beet_examples spatial scenes (flock etc.)
cargo check -p beet --features=thread            # thread scene templates
cargo check --example hello_lambda --features=router,lambda_block,markdown
```

## Instructions

1. Begin a fresh run by overwriting the scratch file:
   ```sh
   : > .agents/tmp/scratch.txt
   ```
2. Walk through sections 1–7 in order, appending each invocation's output:
   ```sh
   timeout 60 cargo run --example hello_world --features=action 2>&1 | tee -a .agents/tmp/scratch.txt
   ```
3. On a failing example, isolate it with `--features=…` matching the workspace declaration, fix using a subagent if the fault is non-trivial, then rerun just that example before moving on.
4. For server examples, launch with `run_in_background`, probe with `curl`, kill the background pid before continuing.
5. After all sections pass, re-grep the scratch file for `error`, `warning`, `panicked`, and `FAIL` to catch anything missed. Fix any warnings encountered.
6. Once the full set passes again, provide a comprehensive summary of what changed and which examples were touched.

## Success

The smoke set passes when every command in sections 1–7 exits 0 (or, for the server probes, the `curl` returns the expected body) and `.agents/tmp/scratch.txt` contains no unexpected `error`/`warning`/`panicked` lines.
