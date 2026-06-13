# Run Examples

Use the workspace examples as smoke tests to catch regressions that unit and integration tests may have missed.

## Context

Not every example is verifiable from the CLI — many are 3D/2D Bevy windowed apps, browser-driven, or TUIs that block on stdin. This skill only exercises the ones that exit on their own (or that we can probe with `curl` while they run in the background).

Stream the entire output of each example into `.agents/scratch.txt` (overwrite for the first command, append for subsequent ones), then grep that file. This avoids reruns when checking multiple things.

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
cargo run --example file_based_routes -- codegen            --features=codegen,http_server,json,markdown,fs,ureq,rustls-tls
cargo run --example file_based_routes -- about              --features=codegen,http_server,json,markdown,fs,ureq,rustls-tls
cargo run --example file_based_routes -- call-add --a=10 --b=20  --features=codegen,http_server,json,markdown,fs,ureq,rustls-tls
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

```sh
cargo run --example hello_ml --features=examples,ml        # logs "NearestSentence chose: Attack Behavior"
```

## Not Verifiable Via CLI (skip)

Documented so future passes don't waste time on them:

- **Spatial:** `flock`, `seek`, `seek_3d`, `hello_animation`, `inverse_kinematics` — Bevy windowed apps.
- **ML windowed:** `fetch`, `frozen_lake_run`.
- **Interactive TUI/stdin:** `thread/chat`, `thread/multi_agent`, `ui/term_input`, `ui/tui`, `ui/state`.
- **Browser required:** `ui/crud`, `ui/syntax_highlighting`, `ui/media_renderer` (interactive output).
- **Needs LLM API key:** `thread/oneshot`, `thread/persistent_chat`, `thread/tool_call`, `thread/self_evolving`, `thread/coding_agent`. Compile-check these but don't run unless `OPENAI_API_KEY` / `BEDROCK_*` env is set.
- **Needs AWS:** `hello_lambda`, `hello_lightsail`, `hello_fargate`, `lifecycle`. Compile-check only.
- **Needs sshd:** `ssh_server`, `ssh_client`, `ssh_tui`.

A pure compile check is still useful for the skipped set:

```sh
cargo check --example fetch              --features=examples,ml
cargo check --example flock              --features=examples,spatial
cargo check --example hello_lambda       --features=router,lambda_block,markdown
cargo check --example chat               --features=thread
```

## Instructions

1. Begin a fresh run by overwriting the scratch file:
   ```sh
   : > .agents/scratch.txt
   ```
2. Walk through sections 1–7 in order, appending each invocation's output:
   ```sh
   timeout 60 cargo run --example hello_world --features=action 2>&1 | tee -a .agents/scratch.txt
   ```
3. On a failing example, isolate it with `--features=…` matching the workspace declaration, fix using a subagent if the fault is non-trivial, then rerun just that example before moving on.
4. For server examples, launch with `run_in_background`, probe with `curl`, kill the background pid before continuing.
5. After all sections pass, re-grep the scratch file for `error`, `warning`, `panicked`, and `FAIL` to catch anything missed. Fix any warnings encountered.
6. Once the full set passes again, provide a comprehensive summary of what changed and which examples were touched.

## Success

The smoke set passes when every command in sections 1–7 exits 0 (or, for the server probes, the `curl` returns the expected body) and `.agents/scratch.txt` contains no unexpected `error`/`warning`/`panicked` lines.
