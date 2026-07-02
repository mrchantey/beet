# Deploy

The release process for the beet website. Validate the SAME site across three environments in sequence, each in its own sub-agent, each with an IDENTICAL verification pass:

1. **Local** proves the build (serve on localhost, verify, stop).
2. **Dev** proves the cloud path (deploy to `dev.beet.org`, verify, then tear down, since a standing dev environment is a real monthly cost).
3. **Prod** publishes the permanent production site (`beet.org` + `www.beet.org`).

Run each step as a SEPARATE sub-agent. Do them in series: Dev only after Local is green, Prod only after Dev is green and torn down.

## DNS topology

The deploy block is stage-aware (`FargateBeetSiteBlock`, `crates/beet_extra/src/infra/templates.rs`):

- `dev` stage publishes ONLY `dev.beet.org`.
- `prod` stage publishes the apex `beet.org` + `www.beet.org`.

Both serve html over http AND a multi-tenant live terminal over ssh; the whole site (`site/main.bsx`, `routes/`, `templates/`, plus the `assets/` bucket) is read from S3 by the generic `beet` binary at runtime. The deployed binary is built with `--features aws_sdk,ssh` (see `<BeetBinaryBuild features="aws_sdk,ssh"/>` in `main.bsx`), so the served site includes the ssh terminal. `aws_sdk` alone would serve http only.

## Commands

Creds load from `.env` (AWS, `CLOUDFLARE_API_TOKEN`/`CLOUDFLARE_ZONE_ID`, `BEET_SSH_HOST_KEY`). The lean `beet-*` recipes use a headless build (no winit/ml) and clear `AWS_PROFILE`; prefer them over `just beet <verb>` (which forces the heavy winit/ml build and can hit a wgpu teardown SIGSEGV).

| intent | command |
| --- | --- |
| local serve (http+ssh) | `cargo run -p beet-cli -- serve site --server=http,ssh` |
| pre-apply safety check | `just beet-validate` then `just beet-plan` (eyeball the plan) |
| dev deploy | `just beet-deploy` (alias: `just beet deploy`) |
| dev destroy | `just beet-destroy` (alias: `just beet destroy --force`) |
| prod deploy | `just beet-deploy --stage=prod` (alias: `just beet deploy --stage=prod`) |

`just beet-plan --stage=prod` shows the prod plan without applying.

## Verification (IDENTICAL for every step)

Each step verifies its environment with the same five checks. Parameters: a `BASE_URL` (`http://localhost:<port>` for local, `https://dev.beet.org` for dev, `https://beet.org` for prod) and an SSH target (`127.0.0.1` + the local ssh port for local; the hostname + port 22 for dev/prod). All five must pass.

### a. curl (raw http)

GET each key page; assert HTTP 200 and the expected marker:

- `/` (home) -> 200, renders the landing page
- `/docs` -> 200
- `/docs/design` -> 200
- `/docs/design/counter` -> 200, body contains `Counter` and `You have clicked`
- `/docs/design/color_schemes` -> 200 (the styles page)
- `/blog` -> 200

Also fetch `/docs/design/counter?color-scheme=light` and `?color-scheme=dark` and confirm 200 (the scheme is applied server-side for the screenshot check below).

### b. playwright interactive (navigability + the counter)

The counter (`site/routes/docs/design/counter.bsx`) is a reactive page: a "More" button increments and a "Less" button decrements a document field rendered as "You have clicked N times." Drive it with playwright (no MCP; use the CLI/library):

- module: `NODE_PATH=/home/pete/.local/lib/node_modules` then `require('playwright')`
- browsers: `PLAYWRIGHT_BROWSERS_PATH=/home/pete/.cache/ms-playwright` (chromium cached)

Script (headless chromium): goto `BASE_URL/docs/design/counter`, wait for `Counter`, read the count, click "More" twice and assert it reaches "You have clicked 2 times", click "Less" and assert "1 times". Also navigate `/` -> a docs page -> the counter via in-page links to confirm the site is actually navigable (not just direct loads). Print `counter OK` on success and exit non-zero on any failed assertion.

### c. playwright screenshot (styles + color schemes)

Screenshot the home page, the counter page, and `/docs/design/color_schemes`, each in default, `?color-scheme=light`, and `?color-scheme=dark`. The CLI is enough: `PLAYWRIGHT_BROWSERS_PATH=/home/pete/.cache/ms-playwright playwright screenshot --url '<BASE_URL>/docs/design/counter?color-scheme=light' .agents/tmp/<step>-counter-light.png`. Save to `.agents/tmp/`, then `Read` the PNGs and confirm they render styled (typography, buttons, layout present; light vs dark visibly differ), not an unstyled or broken page.

### d. ssh (the live terminal + multi-tenancy)

The site is also a navigable charcell TUI over ssh (`SshTuiServer`, multi-tenant). Connect non-interactively over a pty and verify the SAME pages + counter, then prove two clients run at once:

- connect: `ssh -tt -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=15 -p <ssh_port> <user>@<host>` (local: `-p <port> root@127.0.0.1`; dev/prod: port 22, any user. The stable `BEET_SSH_HOST_KEY` gives a consistent fingerprint, so `StrictHostKeyChecking=no` is safe.)
- the handshake completing + a rendered frame appearing confirms the ssh server is up.
- drive the TUI by feeding navigation keystrokes to the pty (with short sleeps between) and capturing the rendered frames: reach the counter page, trigger an increment, and grep the captured output for `Counter` and the incremented count. Discover the exact nav keystrokes during the LOCAL step and record the recipe in the "ssh driver" note below so Dev/Prod reuse it verbatim.
- multi-tenancy: launch TWO ssh sessions at once (both backgrounded), drive both, and confirm BOTH render the site and respond independently with no crash/hang on either.

#### ssh driver (filled in by the Local step)

Use the reusable scripts in `.agents/tmp/deploy-verify/` (built and shaken out locally): `ssh.sh <HOST> <PORT>` runs the whole check (single session + two-client multi-tenancy) and `ssh_pty.py <HOST> <PORT> '<SCRIPT>'` is the underlying pty driver. Local is `ssh.sh 127.0.0.1 <ssh_port>` (default 8339; trust the serve output); dev/prod are `ssh.sh dev.beet.org 22` / `ssh.sh beet.org 22`. The script scales its waits up for any non-localhost host.

Connection (what the driver runs per session): `ssh -tt -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=15 -o LogLevel=ERROR -p <port> root@<host>` on an 80x24 pty.

Navigation recipe. The TUI is a charcell render of the site: a left-click on a sidebar link navigates (focus also moves with Tab/Shift+Tab and Enter activates, but clicks are deterministic). The driver feeds SGR mouse press+release at 1-indexed cells and reconstructs the final frame through a small VT emulator, then greps it. From the home frame, the working sequence to reach the counter and increment is:

1. click "Design" in the sidebar at cell (col 6, row 5): opens the Design subtree and loads `/docs/design`.
2. click "counter" at cell (col 6, row 9): loads `/docs/design/counter`.
3. click the main-area "More" button at cell (col 29, row 13): increments. The sidebar occupies the left column; the page body and its buttons start after the divider at col ~23, so main-area clicks use a high column. "Less" is at (col 38, row 13). Grep the frame for `Counter` and `You have clicked N times`.

As an `ssh_pty.py` script (locally; `w:` is a seconds wait, `m:col,row` a click): `w:4;m:6,5;w:2;m:6,9;w:2;m:29,13;w:2` reaches the counter and clicks More once (expect "clicked 1 times"); append another `m:29,13;w:2` for "2 times".

Two-client recipe: launch two `ssh_pty.py` runs at once (both backgrounded), drive them to DIFFERENT counts (A clicks More once, B twice), `wait`, then assert A shows "clicked 1 times" and B "clicked 2 times" and both rendered `Counter`. Independent counts prove per-session state. `ssh.sh` does exactly this.

KNOWN BLOCKER (multi-tenancy crash): the ssh server has an INTERMITTENT native crash (SIGSEGV/SIGABRT, no Rust panic even with `RUST_BACKTRACE=full`) under two or more CONCURRENT sessions, roughly 20-40% per concurrent session-pair. Single-session navigation is rock solid, and many 2-client runs pass cleanly (the feature itself works: independent state is correct), but a concurrent pair sometimes takes the whole `beet serve` process down (the listener disappears, later connects get "connection refused"). Root cause is in the russh-on-a-multi-threaded-tokio-runtime bridge to the single-threaded bevy world (`crates/beet_net/src/ssh/impl_russh_server.rs` + the `beet_async` world bridge); a session despawn racing the per-frame `ssh_write` drain across sessions is the prime suspect. If `ssh.sh` reports the crash (it detects "connection refused" and says so), restart `beet serve` and re-run; budget a couple of retries. For Dev/Prod the server is a long-lived Fargate task, so a crash means a container restart and a brief blip, not a clean pass. This needs a real fix before the ssh terminal is production-trustworthy under load.

If outbound port 22 is unexpectedly blocked at runtime, fall back to confirming the NLB ssh listener + target-group health via the AWS CLI and record the limitation, but note port 22 was confirmed open from this host at authoring time.

### e. analytics (the cross-transport event log)

The site records analytics for every transport: a server `Request` event per routed request, a `PageView` (with dwell) per web/terminal page visit, and `Click` / `Scroll` / `Error` events from the web client beacon. Verify the visits from checks b-d landed in the store and that any prior data was not lost. Query with the `beet analytics` subcommand (built with `aws_sdk` so `--remote` reads the live DynamoDB table, not a local fallback):

- local: `cargo run -p beet-cli -- analytics summary --dir target/analytics`
- dev/prod: `cargo run -p beet-cli --features infra -- analytics summary --remote --bucket beet-site--<stage>--analytics`

Recipe (run around the b-d checks so the delta is attributable):

1. BASELINE: query once before the b-d checks and record the total (`N events: ...`). A brand-new environment reports `0 events`; an existing one is non-zero.
2. Drive the visits: checks b (playwright: home -> docs -> counter, click "More") and d (ssh: navigate to the counter). These generate page views, a `Click`, and request events.
3. DELTA: query again and assert, from the summary:
   - the total went UP (new events recorded) and is `>=` the baseline (prior events retained, since the store is append/upsert, never truncated).
   - `PageView` events for the visited paths appear under `pages` (eg `/docs/design/counter`).
   - both client kinds are present under `client kinds`: `Web` (the http/playwright visits) and `Terminal` (the ssh session).
   - for dev/prod (geoip enabled in the deploy build), a country appears under `countries` for the web visits.

An automated in-process version of the web half of this flow (http request -> request event, beacon -> page view, prior events retained, the beacon endpoint skipped) lives in `tests/beet_site_analytics.rs`; run it with `cargo test --test beet_site_analytics --features "router,json,fs,testing"` for a fast pre-deploy check of the wiring. The terminal page-view path is unit-tested in `beet_router/src/navigate/navigator.rs`.

## Step 1: Local

```sh
cargo run -p beet-cli -- serve site --server=http,ssh    # run in background
```

Read the bound http + ssh ports from the serve output. Run the full verification (a-e) against `http://localhost:<http_port>` and `127.0.0.1:<ssh_port>`. This step is also the shakedown: settle the exact playwright script and ssh driver here and record them above. Kill the server when done. No cloud or DNS impact.

## Step 2: Dev

```sh
just beet-validate            # resolves, no cloud
just beet-plan                # EYEBALL: dev must touch only beet-site--dev--* and dev.beet.org
just beet-deploy              # build -> tofu -> image -> sync site/ + assets/ -> watch rollout
```

Run the full verification (a-e) against `https://dev.beet.org` (port 22 for ssh), allowing a few minutes for rollout + DNS/ACM to settle (retry with a sane budget). Then ALWAYS tear down:

```sh
just beet-destroy             # removes the dev stack
```

Confirm teardown with `aws s3 ls` + `aws ecr describe-repositories` (no leaked dev resources). Dev is intentionally not left running.

## Step 3: Prod

```sh
just beet-validate --stage=prod
just beet-plan --stage=prod   # EYEBALL: prod creates beet.org + www.beet.org on the prod NLB
just beet-deploy --stage=prod
```

Run the full verification (a-e) against `https://beet.org` (and confirm `https://www.beet.org` serves too; ssh on port 22). LEAVE PROD UP. Teardown, if ever needed, is `just beet-destroy --stage=prod`.

## First-run / migration note

Historically the dev stack published all three hostnames, so `beet.org`, `www`, and `dev.beet.org` currently resolve to the DEV NLB. The first run of this process migrates the apex to prod: Step 2's dev deploy drops the apex from the dev stack (a brief `beet.org` outage), Step 2's destroy removes the dev stack, and Step 3 recreates `beet.org` + `www` on the prod stack. Expect `beet.org` to be down between the dev deploy and the prod deploy completing. If the prod plan reports the apex records already exist (stale), delete or import them before applying. Never run with `--stage=prod` except in Step 3.
