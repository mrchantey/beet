# Deploy

The release process for the beet website. Validate the SAME site across three environments in sequence, each in its own sub-agent, each with an IDENTICAL verification pass:

1. **Local** proves the build (serve on localhost, verify, stop).
2. **Dev** proves the cloud path (deploy to `dev.beet.org`, verify, then tear down, since a standing dev environment is a real monthly cost).
3. **Prod** publishes the permanent production site (`beet.org` + `www.beet.org`).

Run each step as a SEPARATE sub-agent. Do them in series: Dev only after Local is green, Prod only after Dev is green and torn down.

> **Dev MUST be torn down after verification. Only prod stays up.** Dev is a temporary proving ground for the cloud path, not a standing environment: a live dev stack is a real recurring monthly cost. The moment dev verification is green (or fails, or is abandoned), run `just beet-destroy` and confirm no `beet-site--dev--*` S3 buckets, no dev Lightsail instance or static IP, and no `dev.beet.org` DNS record remain. Never leave dev running.

## Topology

The deploy block is stage-aware (`LightsailBeetSiteBlock`, `crates/beet_extra/src/infra/templates.rs`). One `small_3_0` Lightsail box (2 GB, flat monthly price) per stage carries everything: no load balancer, no container registry, no ACM certificate, no VPC of its own.

- **http**: Caddy on the box terminates TLS for every hostname with an automatic Let's Encrypt cert and reverse-proxies to the beet binary on its app port. Cloudflare fronts it PROXIED (an `A` record at the static IP) and runs Full-strict, so the edge verifies Caddy's cert.
- **ssh**: the beet TUI listens on port 22. Cloudflare does not proxy raw TCP, so ssh rides a DNS-ONLY `app.*` hostname pointing straight at the static IP. The box's own management sshd moves to **2222** (reachable with the stack's Lightsail key pair). Note this means the Lightsail browser console, which only dials 22, lands on the beet TUI rather than a shell.
- **DNS** (stage-aware, and this is why `--stage` matters): `dev` publishes ONLY `dev.beet.org` (proxied) + `app.dev.beet.org` (DNS-only). `prod` publishes the apex `beet.org` + `www.beet.org` (proxied) + `app.beet.org` (DNS-only). A `dev` deploy can never touch production apex DNS.

The generic `beet` binary reads the whole site from ONE bucket at runtime, the per-stage app bucket `beet-site--<stage>--app`: `main.bsx`, `routes/`, `templates/` and `assets/` all live in it, the last arriving through the committed `site/assets -> ../assets` symlink the deploy sync follows. No deployed binary reads `beet-site--shared--assets`; that bucket is the developers' source of record, owned by `just beet-shared pull|push` and untouched by any stage deploy. The deployed binary is built with `--features aws_sdk,ssh,geoip` (see `<LightsailBeetSiteBlock features="aws_sdk,ssh,geoip"/>` in `main.bsx`), so the served site includes the ssh terminal and country lookups. `aws_sdk` alone would serve http only.

**The box is REPLACED on every deploy.** The binary, the boot arguments and the service config all render into the instance's `user_data`, which forces replacement, so `just beet-deploy` destroys and recreates the instance (the static IP is re-attached, so the address and DNS are stable). There is no circuit breaker and no automatic rollback: a binary that fails to boot leaves a broken box, not a preserved previous version. Budget several minutes AFTER the instance reports `running` for cloud-init to install Caddy, pull the binary from S3, obtain the Let's Encrypt cert and start the unit.

**`beet-deploy` exiting 0 does NOT mean the site serves.** The deploy's last steps are a timed log tail and a cache purge, neither of which gates on readiness, and cloud-init failures do not propagate back to it. ALWAYS curl the site after the deploy returns. Diagnosis path for a bad origin:

- **Cloudflare `521`** = the edge reached DNS but nothing accepted on the origin's 443. **`526`** = Caddy answered but its cert is not valid for the Full-strict edge.
- Get a real shell on the management sshd (port **2222**, the stack's Lightsail key pair, user `ec2-user`; the private key lives in the tofu state under `aws_lightsail_key_pair`). Then: `systemctl is-active caddy beet-site`, `ss -lntp`, and `sudo grep -iE "error|fail" /var/log/cloud-init-output.log`.
- `beet-site` active on its app port while `caddy` is missing means the TLS terminator failed to install and everything else is fine.

REGRESSION GUARD (fixed): Caddy was installed via its cloudsmith rpm repo, whose setup script writes an `amzn/2023` baseurl that Caddy does not publish, with `skip_if_unavailable=1`. `dnf install -y caddy` therefore exited `No match for argument: caddy`, cloud-init logged a failed `scripts-user` module and carried on, and the box booted serving the app on its own port with NOTHING on 80/443 — while `beet-deploy` reported success and every hostname returned `521`. Caddy is now installed from the pinned upstream static release (`LightsailBlock::CADDY_VERSION`) with its own systemd unit, and the script verifies the binary (`caddy version || exit 1`) so a failed install stops the boot instead of silently producing a TLS-less box. Covered by `installs_caddy_from_static_release_not_rpm` in `crates/beet_infra/src/blocks/lightsail.rs`.

## Commands

Creds load from `.env` (AWS, `CLOUDFLARE_API_TOKEN`/`CLOUDFLARE_ZONE_ID`, `BEET_SSH_HOST_KEY`). Always use the `beet-*` recipes: they build with `--features infra,extra` (without which the deploy routes load as inert tags and the verb does nothing) and clear `AWS_PROFILE`.

| intent | command |
| --- | --- |
| local serve (http+ssh) | `cargo run -p beet-cli -- serve site --server=http,ssh` |
| pre-apply safety check | `just beet-validate` then `just beet-plan` (eyeball the plan) |
| dev deploy | `just beet-deploy` |
| re-publish site only (no redeploy) | `just beet-sync` |
| tail the instance logs | `just beet-watch` |
| dev destroy | `just beet-destroy` |
| prod deploy | `just beet-deploy --stage=prod` |

`just beet-plan --stage=prod` shows the prod plan without applying.

## Verification (IDENTICAL for every step)

Each step verifies its environment with the same five checks. Parameters: a `BASE_URL` (`http://localhost:<port>` for local, `https://dev.beet.org` for dev, `https://beet.org` for prod) and an SSH target (`127.0.0.1` + the local ssh port for local; the DNS-only `app.` subdomain + port 22 for dev/prod, ie `app.dev.beet.org` / `app.beet.org` -- the bare `dev.beet.org`/`beet.org` are Cloudflare-proxied web-only and do NOT forward ssh). All five must pass.

### a. curl (raw http)

GET each key page; assert HTTP 200 and the expected marker:

- `/` (home) -> 200, renders the landing page
- `/docs` -> 200
- `/docs/design` -> 200
- `/docs/design/counter` -> 200, body contains `Counter` and `You have clicked`
- `/docs/design/color_schemes` -> 200 (the styles page)
- `/blog` -> 200

Also fetch `/docs/design/counter?color-scheme=light` and `?color-scheme=dark` and confirm 200 (the scheme is applied server-side for the screenshot check below).

### b. browser verification (navigability, the counter, client errors, mobile layout)

The counter (`site/routes/docs/design/counter.bsx`) is a reactive page: a "More" button increments and a "Less" button decrements a document field rendered as "You have clicked N times." The whole check is one committed test driving the in-house webdriver (`chromedriver` + a chromium on PATH are the only deps):

```sh
BEET_BASE_URL=<BASE_URL> cargo test --test site_browser \
  --features router,json,testing,webdriver -- --include-ignored
```

`tests/site_browser.rs` runs everything below and exits non-zero on any client error or overflow. The checks:

**Client errors (fail on any).** Before navigating, two collectors attach and the run fails if either fires, so a broken client script cannot ship silently -- exactly the miss that let `crypto.randomUUID is not a function` reach the analytics beacon in production:

- `page.console()` -- `console.error` plus uncaught exceptions and failed-request console messages (BiDi `log.entryAdded`).
- `page.responses()` -- any 4xx/5xx subresource. A favicon that fails to load raises no console error, so without this a site whose every asset 403s still reports green.

**Asset sweep.** Load a page that actually carries an image (`/blog/post-6`), collect every `/assets/` reference on it (`img[src]`, `link[href]`, `script[src]`, `source[src]`), fetch each following redirects, and assert 200. This is the store-topology check: the app serves assets from its own bucket, and a private bucket handing out a public url turns every asset into a redirect to a 403. Both this and the response collector were added after exactly that shipped past a green run (`S3Store::public_url` claimed a virtual-hosted url for a private bucket; it now returns `None` unless the store is explicitly `with_public(true)`).

Ignore nothing by default; if a message is genuinely benign, match it exactly and log that it was skipped. CAVEAT: some faults only surface in an insecure context -- `crypto.randomUUID`/`crypto.subtle` are gated to secure contexts, and localhost + `https://` are both secure, so this check does NOT reproduce that specific bug. The durable fix is keeping secure-context-only APIs out of the client (the beacon now derives its id from `crypto.getRandomValues`, available on plain http); the collectors still catch the broad class of client JS errors on every env.

**Counter + navigability.** Headless chromium: goto `BASE_URL/docs/design/counter`, click "More" twice and assert "You have clicked 2 times", click "Less" and assert "1 times" (trusted `performActions` clicks, so hit-testing is real). Then navigate `/` -> `/docs` -> `/docs/design` -> the counter via in-page links at a desktop viewport (the collapsed-nav links are zero-size and unclickable, exactly like a real user; proves the site is navigable, not just direct loads), and load `/blog` + a post (`/blog/post-3`) so the beacon runs on a content page -- the pages the client error was reported on.

**Mobile layout (no horizontal overflow).** At viewports 375x812 and 320x812, for `/`, `/blog`, and `/blog/post-3` assert `document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1`, printing the offending elements on failure so the culprit is obvious. Regression guard: a `<pre>` code block or a wide embed used to blow `<main>` past the viewport (`<main>` is a flex item, fixed with `min-width: 0`), and the header nav overflowed at 320px (fixed with an `@media screen` app-bar `flex-wrap`).

The test asserts ZERO collected client errors across every navigation above; extend `tests/site_browser.rs` if a deploy needs a check it does not cover, rather than scripting around it.

### c. screenshot (styles + color schemes)

Screenshot the home page, the counter page, and `/docs/design/color_schemes`, each in default, `?color-scheme=light`, and `?color-scheme=dark`, via the cli: `cargo run -p beet-cli -- screenshot '<BASE_URL>/docs/design/counter?color-scheme=light' --output=.agents/tmp/<step>-counter-light.png` (an installed `beet` works too). Save to `.agents/tmp/`, then `Read` the PNGs and confirm they render styled (typography, buttons, layout present; light vs dark visibly differ), not an unstyled or broken page. The server-side default scheme is dark, so `default` and `dark` are expected to be identical.

### d. ssh (the live terminal + multi-tenancy)

The site is also a navigable charcell TUI over ssh (`SshTuiServer`, multi-tenant). Connect non-interactively over a pty and verify the SAME pages + counter, then prove two clients run at once:

- connect: `ssh -tt -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=15 -p <ssh_port> <user>@<host>` (local: `-p <port> root@127.0.0.1`; dev/prod: the `app.` subdomain `app.dev.beet.org` / `app.beet.org` on port 22, any user. The bare `dev.beet.org`/`beet.org` records are Cloudflare-proxied and do not forward ssh; only the DNS-only `app.*` record points straight at the box's static IP. The stable `BEET_SSH_HOST_KEY` gives a consistent fingerprint, so `StrictHostKeyChecking=no` is safe.)
- the handshake completing + a rendered frame appearing confirms the ssh server is up.
- multi-tenancy: launch TWO ssh sessions at once (both backgrounded), drive both, and confirm BOTH render the site and respond independently with no crash/hang on either.

Port **2222** on the same host is the box's own management sshd, not the site. Use it (with the stack's Lightsail key pair) to get a real shell for diagnosis: `systemctl status beet-site`, `journalctl -u beet-site`, `systemctl status caddy`.

#### ssh driver

The driver lives beside this skill, in `.agents/skills/deploy/` (it is COMMITTED there on purpose: `.agents/tmp` is gitignored, and an earlier copy kept there silently vanished between deploys). `ssh.sh <HOST> <PORT>` runs the whole check (single session + two-client multi-tenancy) and `ssh_pty.py <HOST> <PORT> '<SCRIPT>'` is the underlying pty driver, python3 stdlib only.

- local: `.agents/skills/deploy/ssh.sh 127.0.0.1 <ssh_port>` (default 8339; trust the serve output)
- dev/prod: `.agents/skills/deploy/ssh.sh app.dev.beet.org 22` / `.agents/skills/deploy/ssh.sh app.beet.org 22`

The script scales its waits up for any non-localhost host.

Connection (what the driver runs per session): `ssh -tt -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=15 -o LogLevel=ERROR -p <port> root@<host>` on an 80x24 pty.

Navigation recipe. The TUI is a charcell render of the site: a left-click on a sidebar link navigates (focus also moves with Tab/Shift+Tab and Enter activates, but clicks are deterministic). The driver feeds SGR mouse press+release at 1-indexed cells and reconstructs the final frame through a small VT emulator, then greps it. At 80x24 the site is in its narrow layout: the home/content pages hide the sidebar behind a hamburger menu (the `三`/`☰` glyph at the top-left), so the sequence opens the menu, navigates, then reopens it once Design has auto-expanded:

1. click the hamburger menu at cell (col 3, row 1): opens the sidebar overlay (Docs/Design/Blog tree).
2. click "Design" at cell (col 6, row 6): loads `/docs/design` and closes the overlay.
3. click the hamburger again at (col 3, row 1): reopens the overlay, now with the Design subtree expanded (button/color_schemes/counter/...) because the current page is under it.
4. click "counter" at cell (col 6, row 10): loads `/docs/design/counter`.
5. click the "More" button at cell (col 9, row 12): increments. On the counter page the sidebar is closed, so the body is full-width and "More" sits at the left; "Less" is at (col 18, row 12). The page title renders fullwidth (`Ｃｏｕｎｔｅｒ`), so an ASCII `grep Counter` will NOT match -- grep the normal-width body line `You have clicked N times` instead (it uniquely identifies the counter page and carries the count).

As an `ssh_pty.py` script (`w:` is a seconds wait, `m:col,row` a click, `k:` keys): locally `w:5;m:3,1;w:2;m:6,6;w:3;m:3,1;w:2;m:6,10;w:3;m:9,12;w:2;m:9,12;w:2` reaches the counter and clicks More twice (expect "clicked 2 times"). Remote hosts double every wait. `ssh_pty.py` accepts `PTY_RULER=1` to print 1-indexed col/row guides for re-discovering cells if the layout shifts, and `PTY_USER` to override the `root` user.

Two-client recipe: launch two `ssh_pty.py` runs at once (both backgrounded), drive them to DIFFERENT counts (A clicks More once, B twice), `wait`, then assert A shows "clicked 1 times" and B "clicked 2 times" and both rendered the counter. Independent counts prove per-session state. `ssh.sh` does exactly this, and calls out the known crash explicitly if a frame comes back "connection refused/closed/reset".

Image recipe (the third check). The TUI draws real rasters over the cell grid with the kitty graphics protocol, and NONE of the above touches that path: the driver used to advertise a flat `xterm-256color` with a zero-pixel pty, and `KittyGraphicsSupport::from_pty` enables graphics only on a kitty/ghostty term name or a non-zero pixel size. A totally broken image path therefore rendered a clean frame and passed. Three things have to line up, and each one silently produces "no image" on its own:

- `PTY_TERM=xterm-kitty`, else the server never transmits at all.
- A window tall enough for the raster's whole cell box (`PTY_COLS=60 PTY_ROWS=40`), since `desired_placements` only places a rect that fits the screen entirely. At the default 80x24 the 1280x960 post-6 image computes to 30 rows and can never be placed.
- The image scrolled into view (32 `k:down`), since it sits well below the fold.

Assert on `PTY_RAW`, not the frame: the VT emulator discards APC sequences, which is exactly where the payload lives. `ssh.sh` greps the raw stream for an `ESC _ G a=t` transmit and reports the byte count (a healthy run is ~2.7MB).

KNOWN (pre-existing, unfixed): a raster taller than the viewport renders as a silent blank hole, no image and no `[image]: alt` marker. `KittyImage::cell_size` clamps the width axis to `max_cols` but nothing clamps rows, so at the default 80x24 ssh window `/blog/post-6` shows a large blank gap. The fix is a `max_rows` clamp with cols re-derived to preserve aspect, bounded by the SCROLLPORT height (viewport minus header/footer), not `viewport.y`.

REGRESSION GUARD (fixed): below the ~90-column breakpoint the sidebar drawer is `display: none` but `position: absolute`, and the charcell layout pass placed out-of-flow nodes without consulting `display`. The invisible drawer therefore kept a full-height rect and won every hit-test in a 20-cell stripe down the left of the screen, so `More` on the counter page was unclickable over ssh (Tab+Enter still worked). Fixed in `crates/beet_ui/src/render/charcell/layout.rs` (hidden subtrees are dropped before positioning, and a node that earns no rect this pass is zeroed rather than left holding a stale one). Covered by `sidebar_collapses_below_breakpoint` there and `narrow_viewport_stays_interactive` in `tests/bsx_site_tui.rs`. The default ssh window is 80x24, squarely in the affected range, so a deploy from a checkout without this fix fails check d.

KNOWN BLOCKER (multi-tenancy crash): the ssh server has an INTERMITTENT native crash (SIGSEGV/SIGABRT, no Rust panic even with `RUST_BACKTRACE=full`) under two or more CONCURRENT sessions. Root cause is in the russh-on-a-multi-threaded-tokio-runtime bridge to the single-threaded bevy world (`crates/beet_net/src/ssh/impl_russh_server.rs` + the `beet_async` world bridge); a session despawn racing the per-frame `ssh_write` drain across sessions is the prime suspect. Originally recorded at roughly 20-40% per concurrent session-pair; the most recent local shakedown ran 7 concurrent pairs back-to-back with zero crashes and zero coredumps, so the rate is likely lower than recorded, but the bug is NOT fixed. If `ssh.sh` reports the crash, restart the server and re-run; budget a couple of retries. On dev/prod the server is a systemd unit, so a crash means the unit restarts and a brief blip rather than a clean pass. This needs a real fix before the ssh terminal is production-trustworthy under load.

If outbound port 22 is unexpectedly blocked at runtime, fall back to confirming the instance is `running` with its ports open via `aws lightsail get-instance-port-states` and record the limitation, but note port 22 was confirmed open from this host at authoring time.

### e. analytics (the cross-transport event log)

The site records analytics for every transport: a server `Request` event per routed request, a `PageView` (with dwell) per web/terminal page visit, and `Click` / `Scroll` / `Error` events from the web client beacon. Verify the visits from checks b-d landed in the store and that any prior data was not lost. Query with the `beet analytics` subcommand (built with `aws_sdk` so `--remote` reads the live DynamoDB table, not a local fallback):

- local: `cargo run -p beet-cli -- analytics summary --dir target/analytics`
- dev/prod: `cargo run -p beet-cli --features infra -- analytics summary --remote --bucket beet-site--<stage>--analytics`

Recipe (run around the b-d checks so the delta is attributable):

1. BASELINE: query once before the b-d checks and record the total (`N events: ...`). A brand-new environment reports `0 events`; an existing one is non-zero.
2. Drive the visits: checks b (the browser test: home -> docs -> counter, click "More") and d (ssh: navigate to the counter). These generate page views, a `Click`, and request events.
3. DELTA: query again and assert, from the summary:
   - the total went UP (new events recorded) and is `>=` the baseline (prior events retained, since the store is append/upsert, never truncated).
   - `PageView` events for the visited paths appear under `pages` (eg `/docs/design/counter`).
   - both client kinds are present under `client kinds`: `Web` (the http/browser visits) and `Terminal` (the ssh session).
   - for dev/prod (geoip enabled in the deploy build), a country appears under `countries` for the web visits. The geoip database (`assets/databases/country.mmdb`, ~8MB) is gitignored, so it reaches the app bucket only through the deploy's `site/assets` symlink walk from a hydrated checkout: run `just beet-shared pull` before deploying, and if `countries` is empty verify `country.mmdb` is present under `s3://beet-site--<stage>--app/assets/databases/`. Since the app bucket carries the assets, a brand-new dev stack now populates countries on its first run like prod does.

REGRESSION GUARD (fixed): the deploy creates `beet-site--<stage>--analytics` from the stack's app name, while the *running binary* derives the same name independently from `PackageConfig::resource_name` — and `site/main.bsx` declared no `binary_name`, so the runtime fell back to the kebab-cased title and wrote to `beet--<stage>--analytics`, a table that does not exist. Every event failed with a DynamoDB `ResourceNotFoundException` logged to `/var/log/beet-site.log` while the site served perfectly, so a fully green a-d reported `0 events` on check `e`. `site/main.bsx` now declares `binary_name="beet-site"`, and `analytics_names_agree` (`crates/beet_extra/src/infra/templates.rs`) pins the deploy's table name against the *real* committed entry rather than a hand-built config. If a summary ever reports `0 events` again, check the box log for `ResourceNotFoundException` before suspecting the emitters.

The local query takes ~50s and prints server log lines to stdout ahead of the summary; `tail` it. Cross-check a remote total against `aws dynamodb scan --table-name beet-site--<stage>--analytics --select COUNT` before reporting apparent data loss: `DynamoStore::list` used to issue one unpaginated `scan`, so it truncated silently at the first 1MB page (~4k rows) and reported that page as the total, and the per-row read fanned out unbounded so connect timeouts were miscounted as unreadable rows. Both are fixed (`dynamo_store.rs` paginates, `BlobStore::GET_ALL_CONCURRENCY` bounds the fan-out), but a wildly varying total across runs is the signature. An automated in-process version of the web half of this flow (http request -> request event, beacon -> page view, prior events retained, the beacon endpoint skipped) lives in `tests/beet_site_analytics.rs`; run it with `cargo test --test beet_site_analytics --features "router,json,fs,testing"` for a fast pre-deploy check of the wiring. The terminal page-view path is unit-tested in `beet_router/src/navigate/navigator.rs`.

## Step 1: Local

```sh
cargo run -p beet-cli -- serve site --server=http,ssh    # run in background
```

Read the bound http + ssh ports from the serve output (defaults 8337 / 8339). Run the full verification (a-e) against `http://localhost:<http_port>` and `127.0.0.1:<ssh_port>`. This step is also the shakedown: run the browser test and settle the ssh driver here, recording any changes above. Kill the server when done. No cloud or DNS impact.

## Step 2: Dev

```sh
just beet-validate            # resolves, no cloud
just beet-plan                # EYEBALL: dev must touch only beet-site--dev--* and dev.beet.org
just beet-deploy              # build -> stores -> sync site/ + assets/ -> replace box -> watch -> purge
```

Run the full verification (a-e) against `https://dev.beet.org` (ssh on `app.dev.beet.org` port 22), allowing several minutes for cloud-init + Let's Encrypt to settle (retry with a sane budget; a `521` means the origin is not serving yet). Then ALWAYS tear down:

```sh
just beet-destroy             # removes the dev stack
```

Confirm teardown: no dev Lightsail instance or static IP (`aws lightsail get-instances`, `aws lightsail get-static-ips`), no `beet-site--dev--*` S3 buckets (both `--app` and the on-demand `--artifacts`), no dev analytics table (`aws dynamodb list-tables`), and no `dev.beet.org` / `app.dev.beet.org` Cloudflare records. A clean teardown leaves ONLY `beet-site--prod--*`, `beet-site--shared--assets` and `beet-state`. Dev is intentionally not left running.

## Step 3: Prod

```sh
just beet-validate --stage=prod
just beet-plan --stage=prod   # EYEBALL: prod creates beet.org + www.beet.org at the prod static IP
just beet-deploy --stage=prod
```

EYEBALL the plan for two things specifically: that `aws_s3_bucket.beet_site__prod__app` and `aws_dynamodb_table.beet_site__prod__analytics` appear only as *refreshes*, never destroys (they carry the site content and the whole analytics history), and that the DNS changes are the ones you expect.

Run the full verification (a-e) against `https://beet.org` (and confirm `https://www.beet.org` serves too; ssh on `app.beet.org` port 22). LEAVE PROD UP. Teardown, if ever needed, is `just beet-destroy --stage=prod`.

### Changing a hostname's record TYPE

Only relevant when a hostname moves between backends whose records differ in type (the Fargate->Lightsail migration moved `beet.org`/`www.beet.org`/`app.beet.org` from `CNAME` at the NLB to `A` at the static IP). Cloudflare rejects an `A` create while a `CNAME` of the same host exists (**error 81054**), and tofu has no ordering edge between destroying the old record and creating the new one, so a single apply races and can fail with the old backend already destroyed and DNS still pointing at it.

Delete the conflicting records first, then deploy immediately:

1. Pre-build so the deploy's build step is a no-op and the window is as short as possible: `cargo-zigbuild build --package beet-cli --bin beet --release --target x86_64-unknown-linux-gnu --no-default-features --features aws_sdk,ssh,geoip`.
2. Delete each conflicting record through the Cloudflare API (`DELETE /zones/$CLOUDFLARE_ZONE_ID/dns_records/<id>`). This starts the outage.
3. `just beet-deploy --stage=prod` straight away.

Two gotchas afterwards:

- **Your local resolver will negative-cache the deleted name** for the zone's SOA minimum (1800s). `beet.org` can look like `NXDOMAIN` locally long after it is healthy worldwide. Confirm with `resolvectl query --cache=no beet.org`, a public resolver (`https://1.1.1.1/dns-query?name=beet.org&type=A`), or `curl --resolve` before believing a failure — and do not run check b until local resolution is back, since the browser needs it.
- **The box is slow for its first few minutes.** On 2 CPUs, direct-origin requests intermittently stall while cloud-init finishes and Let's Encrypt issues. Re-probe before concluding a fault.

## Before any deploy

1. **Hydrate the assets** (`just beet-shared pull`). The site sync mirrors `site/` with `delete=true` and follows the `site/assets` symlink, so it publishes whatever the checkout holds. `SyncS3Bucket::assert_mirrorable` refuses a missing or empty symlinked child rather than emptying the bucket, but a *stale* tree syncs happily and silently ships old assets.
2. **The apply runs once per layer, and the order matters.** `<TofuApply layer="storage"/>` brings up the stores (the addresses blocks declare under the `storage` layer, an overridable per-block field), the `<DirSync/>` fills them, then a bare `<TofuApply/>` converges the whole stack and replaces the box against content that is already published. A single apply built the instance first, and it booted with `no entry document found in the --store backend`. Naming a layer no block declares is a loud error, never a silent skip. Since the box is replaced rather than rolled, there is no health gate and no rollback: `LightsailWatch` is a CloudWatch log tail, not a readiness check, so re-check the site after the deploy returns, not during.
3. Never run with `--stage=prod` except in Step 3.
