---
name: test-examples
description: Run the workspace example smoke set to catch regressions unit tests miss, covering every feature gate combination with self-terminating examples and curl-probed servers. Use when asked to run or verify the examples.
---

# Run Examples

Use the workspace examples as smoke tests to catch regressions that unit and integration tests may have missed.

## Context

Not every example is verifiable from the CLI — many are 3D/2D Bevy windowed apps, browser-driven, or TUIs that block on stdin. This skill only exercises the ones that exit on their own (or that we can probe with `curl` while they run in the background).

Stream the entire output of each example into `.agents/tmp/scratch.txt` (overwrite for the first command, append for subsequent ones), then grep that file. This avoids reruns when checking multiple things.

Treat compile failures the same as the `test-run` skill: retry once, and if a mold linker error persists (`RUST_MIN_STACK`, "section sizes" etc) bump the workspace `version = "0.0.9-dev.N"` in `Cargo.toml`.

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

### 2. Scripting (`--features=quickjs`)

```sh
cargo run --example scripting --features=quickjs             # JS Script<I,O>
```

### 3. Router (`--features=router,markdown` / plus extras)

CLI router server, persisted router, and the codegen pipeline.

```sh
cargo run --example router           --features=router,markdown
cargo run --example router           --features=router,markdown -- about
cargo run --example cli              --features=router,quickjs -- greet --name=world
# the persisted scene caches the route scripts, so regenerate it after any change
# to script authoring or to a registered type
cargo run --example router_serde     --features=router,quickjs,template_serde -- --new
cargo run --example router_serde     --features=router,quickjs,template_serde
cargo run --example router_serde     --features=router,quickjs,template_serde -- greet --name=world
# rsx_site is a crate, not a root example: generate its routes, then serve. It
# scans typed pages, markdown content and a server action from three collections.
cargo run -p rsx_site --no-default-features --features codegen   # regenerate src/codegen/
cargo run -p rsx_site                                            # http server (default)
cargo run -p rsx_site --features cli -- guide --accept=text/html # render one route to stdout
```

### 4. Todo (`--features=router,json`)

Round-trip the todo document: list → create → list → delete → list, ending on `[]` so `examples/todo/todos.json` is left as found. Keep the `--body` single-quoted wherever the command is scripted: a word-split body routes to `/create/test",..` instead. A missing route (`-- nope`) answers the markdown not-found page and exits 1, like every beet CLI.

```sh
cargo run --example todo --features=router,json -- list
cargo run --example todo --features=router,json -- create --body='{"description":"smoke test","done":false}'
cargo run --example todo --features=router,json -- list
cargo run --example todo --features=router,json -- delete --body=0
cargo run --example todo --features=router,json -- list
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

The `examples,ml` feature only gates windowed scene code (now scene modules in `beet_extra`, not runnable `--example` targets), so there is no self-terminating CLI smoke here. The runtime ML smoke lives in the crate (`hello_ml_basic`, section 6); this feature's compilation is covered by the skip-set check below (and is the only coverage, since `beet_extra` is excluded from the test crates).

### 8. BSX scenes (`beet --entry=<file>.bsx`)

The no-code `.bsx` scenes run through the installed beet CLI (when editing rust, `cargo run -p beet-cli --features=.. -- <args>` instead, so the scenes run against the working tree). Each entry documents its own `beet --entry=..` command in its header, and an entry that declares its hard requirements with `<RequireCfg>` fails fast on a leaner binary, naming what is missing. The self-terminating ones render and exit:

A documented command never carries `--features`: that is the entry's own `<RequireCfg>`'s job. The binary still has to *link* the capability though, and the demo scenes name actions from `beet_extra`, which is the `extra` cargo feature. Build the CLI once with what the set needs and run everything against it:

```sh
cargo build -p beet-cli --features=extra        # every scene below except the ml one
cargo build -p beet-cli --features=extra,ml     # adds `hello_ml.bsx`
```

Every action scene naming a `beet_extra` action (`hello_world`, `simple_action`, `long_running`, `malenia`, `state_machine`, `scripting`) declares `<RequireCfg cfg="feature:extra"/>` beside its root element, where the entry prescan consumes it rather than building it into the `<Sequence>`, so a binary without `extra` refuses the load before anything builds, logging `this binary does not satisfy <RequireCfg cfg="feature:extra"/>` with the unmet `feature:extra` on the next line. `behavior_tree`, `repeat_while`, `utility_ai` and `world_script` use only the core vocabulary and run on any build. `winit` enables `extra`, so the windowed build satisfies the requirement too.

The `ml` build pulls `winit` and bevy_render, so every scene brings up a wgpu device and compiles bevy's compute pipelines on its async compute pool whether or not it needs a GPU; a short scene exits while those compiles are still in the driver. A segfault inside `libnvidia-glcore` during `create_compute_pipeline` (or an `NVVM compilation failed` + `DeviceLost` log) is an exit tearing the driver down beneath that thread, which `process_ext::exit` avoids by `_exit`ing. It is a regression, never host noise: `__GL_SHADER_DISK_CACHE=0` forces every compile through the driver and reproduces it on every short scene. The `extra`-only binary is still the faster build for everything but the ml scene.

```sh
beet --entry=examples/hello                                       # prints "hello world"
beet --entry=examples/action/behavior_tree.bsx                    # sequence + log
beet --entry=examples/ml/hello_ml.bsx                             # logs "NearestSentence chose: ..."
beet --entry=examples/calculator/main.bsx --server=cli add --a=3 --b=4   # result: 7
```

The rest of `examples/action/*.bsx` (`hello_world`, `simple_action`, `long_running`, `repeat_while`, `state_machine`, `utility_ai`, `scripting`, `world_script`) are also self-terminating and worth a sweep.

Skip: `examples/spatial/*.bsx`, `examples/action/wgpu-action.bsx` and `examples/ml/frozen_lake_*.bsx` (windowed), `examples/thread/*.bsx` (need an LLM key), `examples/bsx_site/main.bsx` (HTTP server; verify with `beet --entry=examples/bsx_site --server=cli` instead, whose markdown page must keep its nav links apart and read `© BSX Site <year> v<version> | stage=dev` in the footer). Its served page boots `/wasm/beet-ui.wasm`, a build output under `target/wasm` rather than a synced asset, so serving it over http for a browser needs `just build-wasm-ui` first; `just test-browser` builds it itself.

The `examples/infra/*.bsx` deploy entries are never run (they deploy), but each opts into `RejectUnknownParams`, so its `--help` is the offline check that the declared-params union renders: every entry exits 0 listing its routes and their flags, `bucket.bsx`'s listing `--s3-backend` (its `StateBackendToggle`). Only ever pass `--help` at the root, never a route path, which is a real deploy. `custom_handle_domain.bsx` also requires `feature:atproto`, so a binary without it refuses it naming that.

```sh
cargo build -p beet-cli --features=infra,extra,atproto
for f in examples/infra/*.bsx; do beet --entry=$f --help > /dev/null || echo "FAIL $f"; done
```

Every other scene in `examples/action/` exits 0 (`wgpu-action.bsx` requires `feature:winit`, so a leaner binary refuses it before anything builds). Verify a refusal once per sweep against a binary built without `extra` (`cargo build -p beet-cli`): `hello_world.bsx` must exit nonzero naming `feature:extra`. A `() -> Outcome` load exits zero once it resolves whatever the outcome (an outcome is a branch, not an error), so `malenia.bsx` and `repeat_while.bsx`, whose `<Repeat>` ends by returning its body's fail, report a completed run. Only a scene carrying `{OutcomeOverload{error_on_fail:true}}` turns a `Fail` into a nonzero exit.

## Not Verifiable Via CLI (skip)

Documented so future passes don't waste time on them:

- **Spatial / ML scenes:** `flock`, `seek`, `fetch`, `frozen_lake_run` etc. are no longer `--example` targets — they live as scene modules in `beet_extra`, reached only by building the `examples,spatial` / `examples,ml` features.
- **Thread scenes:** `chat`, `multi_agent`, `oneshot`, `persistent_chat`, `tool_call`, `self_evolving`, `coding_agent` are `.bsx` markup scenes under `examples/thread/`, not `--example` targets. Several also need an LLM key (`OPENAI_API_KEY` / `BEDROCK_*`).
- **Interactive TUI/stdin:** `ui/term_input`, `ui/tui`, `ui/state` — real examples that block on stdin.
- **Browser required:** `ui/crud`, `ui/syntax_highlighting`, `ui/media_renderer` (interactive output).
- **Needs sshd:** `ssh_server`, `ssh_client`, `ssh_tui`.

A pure compile check is still useful for the skipped set. The first three cover `beet_extra` and the feature gates, which no test crate compiles:

```sh
cargo check -p beet --features=examples,ml       # beet_extra ML scenes (fetch etc.)
cargo check -p beet --features=examples,spatial  # beet_extra spatial scenes (flock etc.)
cargo check -p beet --features=thread            # thread scene templates
cargo check -p beet-cli --features infra         # examples/infra/*.bsx deploy templates
```

## Instructions

1. Begin a fresh run by overwriting the scratch file:
   ```sh
   : > .agents/tmp/scratch.txt
   ```
2. Walk through sections 1–8 and the skip-set compile checks in order, appending each invocation's output:
   ```sh
   timeout 60 cargo run --example hello_world --features=action 2>&1 | tee -a .agents/tmp/scratch.txt
   ```
3. On a failing example, isolate it with `--features=…` matching the workspace declaration, fix using a subagent if the fault is non-trivial, then rerun just that example before moving on.
4. For server examples, launch with `run_in_background`, probe with `curl`, kill the background pid before continuing.
5. After all sections pass, re-grep the scratch file for `error`, `warning`, `panicked`, and `FAIL` to catch anything missed. Fix any warnings encountered.
6. Once the full set passes again, provide a comprehensive summary of what changed and which examples were touched.

## Success

The smoke set passes when every command in sections 1–8 and the skip-set compile checks exit 0 (or, for the server probes, the `curl` returns the expected body) and the scratch output contains no `error`/`warning`/`panicked` lines beyond the `tracing` example's own demo `WARN`/`ERROR`. Every other one is fixed, whoever wrote the code it points at.
