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

The generic `beet` binary reads the whole site from ONE bucket at runtime, the per-stage app bucket `beet-site--<stage>--app`: `main.bsx`, `routes/`, `templates/` and `assets/` all live in it, replicated from the checkout's `site/` by the deploy sync. No deployed binary reads `beet-site--shared--assets`; that bucket is the developers' source of record for `site/assets`, owned by `just site-shared pull|push` and untouched by any stage deploy. (The separate `beet--shared--assets` bucket backs the WORKSPACE `./assets` tree the examples, tests and wasm builds read; `just beet-shared pull|push`. The two trees are not mirrors: `blog/` and `branding/` belong to the site, and a few workspace-built files — the wasm binary, the geoip database, the robot faces — are borrowed into `site/assets` by the `<DirCopy/>` manifest in `site/main.bsx`, which runs ahead of every publish.) The deployed binary is built with `--features aws_sdk,ssh,geoip` (see `<LightsailBeetSiteBlock features="aws_sdk,ssh,geoip"/>` in `site/main.bsx`), so the served site includes the ssh terminal and country lookups. `aws_sdk` alone would serve http only.

**The site entry declares its own infrastructure.** `site/main.bsx` is a one-shot `<CliServer{always:true}>` dispatcher whose site is the `serve` route (the box launches `app --store=s3://.. --server=http,ssh serve`, and a developer launches `beet --main=site serve --server=http`: one process, one argv, the same verb). Its analytics table is declared ONCE, under the stage `<Stack>`, as `<DynamoTableBlock bx:ref="analytics" label="analytics"/>`: the deploy provisions it and the runtime records to it through `StoreRef($analytics)` (a `bx:ref` is document-global, so the serving router reaches it from another branch), both composing `<app>--<stage>--analytics` from the `app_name` on the entry's `<PackageConfig/>`. The deploy verbs load in every binary: the deployed one (which links neither the infra nor the extra crates) builds them as inert `UnregisteredTag` entities, and the `<Stack>`'s `RequireFeatures(["infra","extra"])` fails any dispatch into them naming the missing features. A `Router` is a url space, so the site's urls stay rooted at `/` and no http/ssh request can reach a deploy route (`curl /deploy` -> 404).

**The box is replaced only when the BOX changes.** The instance's `user_data` renders machine config alone (the blueprint, Caddy, the sshd relocation, the CloudWatch agent, the systemd unit), never a version, so a code-only deploy plans no change to `aws_lightsail_instance`. The binary instead reaches the box through the artifacts bucket's stable release pointer, `s3://beet-site--<stage>--artifacts/current/main-lightsail.env`, which names this deploy's id and its versioned artifact key. The unit's `ExecStart` is a launcher (`/usr/local/bin/beet-site-run`) that re-reads that pointer at every start, so `systemctl restart` IS the deploy, and `<LightsailRelease/>` (after `<TofuApply/>`) performs it over the management sshd on 2222 and gates on the box actually serving: the RUNNING process reports this deploy's id (read from `/proc/<MainPID>/environ`), one request through the app's own port on loopback is answered, and `NRestarts` is unchanged across that whole check. A fetch that fails leaves the installed binary serving rather than killing the unit.

Editing `LightsailBlock::build_user_data` (bumping `CADDY_VERSION`, changing the unit, moving the management port) IS a rebuild: terraform cannot update `user_data` in place, so the instance is destroyed and recreated, and the IAM access key rotates with it (the rotation trigger is keyed on a digest of the rendered script, so the key rotates with every machine-config change; a rebuild from a non-script change, ie a bundle resize, keeps its key). Budget several minutes AFTER the instance reports `running` for cloud-init to install Caddy, pull the binary from S3, obtain the Let's Encrypt cert and start the unit; the static IP re-attaches so the address and DNS are stable. There is still no circuit breaker and no automatic rollback: a binary that fails to boot leaves a broken box. Rolling back is re-pointing the release pointer (`beet rollback`, which rewrites it) and restarting the unit.

This matters beyond the outage window. `small_3_0` is burstable at a **20% baseline per vCPU**, and a fresh instance starts at ZERO burst capacity, so a rebuild does the heaviest work of the instance's life while clamped to baseline. That is what produced the cluster of AWS SDK `dispatch failure` errors within ~75 seconds of unit start on a freshly deployed box. Covered by `code_only_deploy_renders_one_box` and `machine_config_change_rebuilds_and_rotates` in `crates/beet_infra/src/blocks/lightsail.rs`.

**`beet-deploy` exiting 0 does NOT mean the SITE serves.** The deploy's last steps are a timed log tail and a cache purge, neither of which gates on readiness, and cloud-init failures do not propagate back to it. `<LightsailRelease/>` now proves the APP serves — an `active` unit carrying this deploy's id that answers a request on its own port with no restart underneath — so the old failure where a crash-looping box reported `is serving release ..` after ~340 restarts is closed, and a deploy exiting 0 while `LightsailWatch` prints a fatal boot error in that same log should not recur. What the gate deliberately cannot see is everything in FRONT of the app: it probes loopback, so Caddy, DNS and certificate state are all still unproven. ALWAYS curl the site after the deploy returns. Diagnosis path for a bad origin:

- **Cloudflare `521`** = the edge reached DNS but nothing accepted on the origin's 443. **`526`** = Caddy answered but its cert is not valid for the Full-strict edge.
- Get a real shell on the management sshd (port **2222**, the stack's Lightsail key pair, user `ec2-user`). Every deploy materialises that key beside the rendered config as `target/infra/beet-site/deploy_key.pem` (mode 600) for `<LightsailRelease/>`'s own ssh, so reach for it rather than digging the tofu state's `aws_lightsail_key_pair` out by hand: `ssh -i target/infra/beet-site/deploy_key.pem -p 2222 ec2-user@app.<stage-host>`. It is the stage of the LAST deploy, so re-run the deploy's stage before trusting it. Then: `systemctl is-active caddy beet-site`, `ss -lntp`, and `sudo grep -iE "error|fail" /var/log/cloud-init-output.log`.
- `beet-site` active on its app port while `caddy` is missing means the TLS terminator failed to install and everything else is fine.

REGRESSION GUARD (fixed): the deploy shipped a binary it never built. A block is declared under its `<Stack>` rather than as a step in the deploy sequence, so nothing dispatches it and its `BuildArtifact`'s action never ran; `<TofuApply/>` went on reading that artifact's file off disk, hashing it and uploading it, so every deploy silently shipped whatever an earlier deploy had left in `target/` (and on a machine with no prior artifact would fail on a missing file instead). The box then crash-looped on `failed to load entry main.bsx: no component, resource or template registered for tag Stack`, warning that types newer than the stale binary were unregistered, while `beet-deploy` reported success. Building now belongs to the step that CONSUMES the artifact: `TofuApplyAction` calls `BuildArtifact::build()` immediately before reading each artifact's bytes, so an artifact that is uploaded but never built is unrepresentable. Covered by `build_runs_the_declared_process` in `crates/beet_infra/src/actions/build_artifact.rs`. A deploy log with no `building: cargo-zigbuild ..` line is this bug back.

REGRESSION GUARD (fixed): Caddy was installed via its cloudsmith rpm repo, whose setup script writes an `amzn/2023` baseurl that Caddy does not publish, with `skip_if_unavailable=1`. `dnf install -y caddy` therefore exited `No match for argument: caddy`, cloud-init logged a failed `scripts-user` module and carried on, and the box booted serving the app on its own port with NOTHING on 80/443 — while `beet-deploy` reported success and every hostname returned `521`. Caddy is now installed from the pinned upstream static release (`LightsailBlock::CADDY_VERSION`) with its own systemd unit, and the script verifies the binary (`caddy version || exit 1`) so a failed install stops the boot instead of silently producing a TLS-less box. Covered by `installs_caddy_from_static_release_not_rpm` in `crates/beet_infra/src/blocks/lightsail.rs`.

AN INTERRUPTED APPLY LEAVES TWO MESSES, AND BOTH BLOCK THE RETRY. A deploy killed mid-apply (SIGTERM, a lost terminal, a timeout) leaves (1) a stale state lock and (2) resources that exist in AWS but not in state, because tofu creates them before it persists. The retry then fails twice over: first `Error acquiring the state lock` naming a lock id and the time it was `Created`, then a wall of `EntityAlreadyExists` / `ResourceAlreadyExistsException`. Neither is a code fault and neither self-heals. Recovery, in order:

1. Confirm nothing is actually running (`pgrep -af tofu`), then `tofu -chdir=target/infra/<app> force-unlock -force <LOCK_ID>` using the id from the error. The lock is an object in the state bucket (`<key>.tflock`); deleting it by hand works but skips the id check.
2. Diff `tofu -chdir=target/infra/<app> state list` against what AWS actually holds, and `tofu import <address> <id>` each orphan. Prefer import to deleting and recreating whenever the resource is live (a schedule that has fired, a log group holding evidence). The ids are per-type: a log group is its name, an IAM role its role name, an inline role policy `<role>:<policy>`, a policy attachment `<role>/<policy-arn>`, a lambda its function name, a scheduler schedule `<group>/<name>` (ie `default/<name>`).
3. Re-run the deploy; it should now converge to no-op plus whatever genuinely changed.

CHECK THE REAL EXIT CODE. `just beet-deploy ... | sed ...; echo $?` reports the exit of `sed`, so a deploy killed by a signal reads as success. Redirect instead of piping (`just beet-deploy --stage=prod > log 2>&1; echo "EXIT=$?"`) and strip the ANSI afterwards, or set `-o pipefail`. `just` prints `recipe ... was terminated ... by signal 15` on the line above, which is the real tell.

AN APPLY OUTLIVES MOST TOOL DEADLINES; RUN IT DETACHED. A full deploy takes 10-15 minutes, and a deploy driven from a tool with a kill deadline (an agent harness, a CI timeout) can SIGTERM the mid-flight apply, leaving exactly the two messes above. Run it detached and watch the log instead: `setsid nohup sh -c 'just beet-deploy --stage=prod > .agents/tmp/deploy.log 2>&1; echo "EXIT=$?" >> .agents/tmp/deploy.log' &`, then tail the log until the `EXIT=` line appears.

REGRESSION GUARD (fixed): the release step passed a box that never served. `<LightsailRelease/>` confirmed only that the unit was `active` and that `/proc/<MainPID>/environ` carried this deploy's id — but under `Restart=always`/`RestartSec=3` a crash-looping unit ALWAYS has a live `MainPID` carrying the right id, so a deploy exited 0 while the box 502'd and restarted ~340 times. The converge loop is now the gate: an attempt passes only when the unit is active, the running process reports the expected id, `curl -fsS --max-time 5 http://127.0.0.1:<app_port>/` succeeds, AND `NRestarts` is unchanged across the whole attempt (a crash loop that answers one lucky request between restarts still fails). The probe is loopback deliberately — with a domain the firewall opens 80/443 only, so the app port is reachable from the box alone, which is exactly "the app itself serves". Covered by `release_proves_the_running_process` and `release_probes_the_declared_port` in `crates/beet_infra/src/blocks/lightsail.rs`. A release step reporting success on a box whose `NRestarts` is nonzero is this bug back.

## Commands

Creds load from `.env` (AWS, `CLOUDFLARE_API_TOKEN`/`CLOUDFLARE_ZONE_ID`, `BEET_SSH_HOST_KEY`). Always use the `beet-*` recipes: they build with `--features infra,extra` (without which the deploy routes load as inert tags and the verb does nothing) and clear `AWS_PROFILE`.

| intent | command |
| --- | --- |
| local serve (http+ssh) | `cargo run -p beet-cli -- --main=site serve --server=http,ssh` |
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
2. click "Design": loads `/docs/design` and closes the overlay.
3. click the hamburger again at (col 3, row 1): reopens the overlay, now with the Design subtree expanded (button/color_schemes/counter/...) because the current page is under it.
4. click "counter": loads `/docs/design/counter`.
5. click the "More" button: increments. On the counter page the sidebar is closed, so the body is full-width and "More" sits at the left, with "Less" beside it. The page title renders fullwidth (`Ｃｏｕｎｔｅｒ`), so an ASCII `grep Counter` will NOT match -- grep the normal-width body line `You have clicked N times` instead (it uniquely identifies the counter page and carries the count).

**Everything but the hamburger is clicked BY LABEL, not by cell.** The sidebar lists every doc page and every blog post, so adding one page shifts every row beneath it and a pinned `m:col,row` clicks its neighbour instead -- silently, since the driver has no idea it missed. That is exactly what happened in the Phase 1 shakedown: the `Crates` docs section landed above `Design`, so the pinned `(6,6)` opened `/docs/crates/beet_core` and all three checks failed on a page they never asked for, while `(6,15)` in the image check reached the wrong blog post. `ssh_pty.py` therefore takes `t:<text>`, which locates `text` in the CURRENT frame (row-major, cell-aligned so a preceding fullwidth title cannot skew the column) and clicks its first cell, exiting with the frame if it is absent. The hamburger keeps its cell, being the one control no content can move.

As an `ssh_pty.py` script (`w:` is a seconds wait, `m:col,row` a cell click, `t:` a label click, `k:` keys): locally `w:5;m:3,1;w:2;t:Design;w:3;m:3,1;w:2;t:counter;w:3;t:More;w:2;t:More;w:2` reaches the counter and clicks More twice (expect "clicked 2 times"). Remote hosts double every wait. `ssh_pty.py` accepts `PTY_RULER=1` to print 1-indexed col/row guides, for reading the layout when a label match is ambiguous, and `PTY_USER` to override the `root` user.

Two-client recipe: launch two `ssh_pty.py` runs at once (both backgrounded), drive them to DIFFERENT counts (A clicks More once, B twice), `wait`, then assert A shows "clicked 1 times" and B "clicked 2 times" and both rendered the counter. Independent counts prove per-session state. `ssh.sh` does exactly this, and calls out the known crash explicitly if a frame comes back "connection refused/closed/reset".

Image recipe (the third check). The TUI draws real rasters over the cell grid with the kitty graphics protocol, and NONE of the above touches that path: the driver used to advertise a flat `xterm-256color` with a zero-pixel pty, and `KittyGraphicsSupport::from_pty` enables graphics only on a kitty/ghostty term name or a non-zero pixel size. A totally broken image path therefore rendered a clean frame and passed. Three things have to line up, and each one silently produces "no image" on its own:

- `PTY_TERM=xterm-kitty`, else the server never transmits at all.
- Nothing about the window. The check runs at the DEFAULT 80x24: sizing contains a raster within its scroll port and `desired_placements` places the visible part of a partly clipped one, so the default window draws the post-6 image. It used to ask for `PTY_COLS=60 PTY_ROWS=40`, which is now gone.
- The image scrolled into view (32 `k:down`), since it sits well below the fold. The post itself is reached by label (`t:Folk Technology`, post-6), for the same reason the navigation recipe is.

Assert on `PTY_RAW`, not the frame: the VT emulator discards APC sequences, which is exactly where the payload lives. `ssh.sh` greps the raw stream for an `ESC _ G a=t` transmit and reports the byte count (a healthy run is ~2.7MB), then for an `a=p` placement carrying the `w=`/`h=` source-rect keys, which ride a placement ONLY when the raster is cropped. The second grep is the standing guard on the blank-hole class below: a placement that never crops means either that regression is back or the raster no longer straddles the port, and both want looking at.

FIXED, AND CONFIRMED LIVE (Phase 10): a raster taller than its scroll port used to render as a silent blank hole, no image and no `[image]: alt` marker. `KittyImage::cell_size` clamped the width axis to the available columns but nothing bounded rows, so the 1280x960 post-6 photo computed to 30 rows at the default 80x24 window, and `desired_placements` dropped any rect the screen or clip did not contain whole. Sizing now bounds rows by the nearest scroll port's height and re-derives the columns to hold the aspect (`CellBounds`, threaded through the measure and layout passes), and a partly clipped placement draws its visible portion through the protocol's source rect instead of disappearing. Covered by `tall_raster_fits_the_default_window`, `raster_bounds_to_its_nested_scroll_port`, `scrolled_raster_places_a_moving_crop` and `every_viewport_places_the_raster` in `crates/beet_ui/src/render/charcell/kitty.rs`. Live over ssh at the default 80x24 the photo places as `c=64,r=23` (bounded by its port, not the 30 rows aspect alone would give) and crops as it scrolls, the source rect walking `h=760` -> `h=920` up to the port's bottom edge and then `y=40` -> `y=120` down the raster.

REGRESSION GUARD (fixed): below the ~90-column breakpoint the sidebar drawer is `display: none` but `position: absolute`, and the charcell layout pass placed out-of-flow nodes without consulting `display`. The invisible drawer therefore kept a full-height rect and won every hit-test in a 20-cell stripe down the left of the screen, so `More` on the counter page was unclickable over ssh (Tab+Enter still worked). Fixed in `crates/beet_ui/src/render/charcell/layout.rs` (hidden subtrees are dropped before positioning, and a node that earns no rect this pass is zeroed rather than left holding a stale one). Covered by `sidebar_collapses_below_breakpoint` there and `narrow_viewport_stays_interactive` in `tests/bsx_site_tui.rs`. The default ssh window is 80x24, squarely in the affected range, so a deploy from a checkout without this fix fails check d.

KNOWN, AND NOT SEEN SINCE (multi-tenancy crash): an intermittent native crash (SIGSEGV/SIGABRT, no Rust panic even with `RUST_BACKTRACE=full`) was recorded under two or more CONCURRENT ssh sessions, originally at roughly 20-40% per session pair, and suspected in the russh-on-a-tokio-runtime bridge to the single-threaded bevy world, ie a session despawn racing the per-frame `ssh_write` drain across sessions. Treat the 20-40% figure as STALE: the crash has not been observed since, across ~17 live session pairs (local, dev and prod, including three concurrent sessions streaming a 2.7MB image) and every run of the harness below.

Phase 6 built the instrument that was missing, and it found a real defect on exactly the suspected path. A connection entity's lifetime belongs to its accept loop, which despawns it the moment the client's channel closes; an entity world scope does not flush, so the surface insert queued by the pty observer was still pending when that despawn ran, and `EntityWorldMut::despawn` flushes the command queue itself, AFTER removing the entity. Every client that vanished between its pty request and the next flush therefore raised "Entity despawned" through the app's error handler, ie a panic on the server's world thread, and a public port 22 supplies that shape continuously in the form of scanners. The harness hit it ~500 times per 2000-session run. Every command the ssh path aims at a connection is now silenced (`try_insert` / `try_trigger_target` / `try_remove`), pinned by `a_session_that_vanishes_before_its_surface_is_not_an_error` (`crates/beet_router/src/navigate/ssh_tui_server.rs`) and `try_trigger_target_tolerates_a_despawned_target` (`crates/beet_core/src/bevy_utils/entity_target_event.rs`); both fail without it.

The standing instrument is `ssh_stress_multi_tenancy`, at the bottom of `crates/beet_router/src/navigate/ssh_tui_server.rs`: 2000 real russh sessions through a real listener, 24 concurrent, each dropping its socket while its neighbours are mid-frame, one in four vanishing before its own page has finished building. It is `#[ignore]`d, so the sanctioned test run compiles it and skips it. Run it with

	cargo test -p beet_router --features=ssh_stress --lib -- --ignored ssh_stress

18,000 session cycles after the fix (12,000 across six debug runs and 6,000 across three release runs, release being what the box runs) produced zero crashes, zero coredumps and zero raised errors. The roughly 10,000 cycles run before the fix produced the same zero crashes and zero coredumps, and ~500 raised errors per run.

This does NOT prove the historical crash is gone. The harness renders a synthetic page and never exercises the kitty image path's multi-megabyte transmits, and the world-thread blocking fixes that landed earlier may account for the silence on their own. So `systemctl show beet-site -p NRestarts` stays the free canary on every deploy, and `ssh.sh` still calls the crash out by name if a frame comes back "connection refused/closed/reset". A sighting now is new information: capture the coredump with `coredumpctl` BEFORE restarting the unit.

If outbound port 22 is unexpectedly blocked at runtime, fall back to confirming the instance is `running` with its ports open via `aws lightsail get-instance-port-states` and record the limitation, but note port 22 was confirmed open from this host at authoring time.

### e. analytics (the cross-transport event log)

The site records analytics for every transport: a server `Request` event per routed request, a `PageView` (with dwell) per web/terminal page visit, and `Click` / `Scroll` / `Error` events from the web client beacon. Verify the visits from checks b-d landed in the store and that any prior data was not lost. Query with the `beet analytics` subcommand (built with `aws_sdk` so `--remote` reads the live DynamoDB table, not a local fallback):

- local: `cargo run -p beet-cli -- analytics summary --dir target/stores/analytics`
- dev/prod: `cargo run -p beet-cli --features infra -- analytics summary --remote --bucket beet-site--<stage>--analytics`

Recipe (run around the b-d checks so the delta is attributable):

1. BASELINE: query once before the b-d checks and record the total (`N events: ...`). A brand-new environment reports `0 events`; an existing one is non-zero.
2. Drive the visits: checks b (the browser test: home -> docs -> counter, click "More") and d (ssh: navigate to the counter). These generate page views, a `Click`, and request events.
3. DELTA: query again and assert, from the summary:
   - the total went UP (new events recorded) and is `>=` the baseline (prior events retained, since the store is append/upsert, never truncated).
   - `PageView` events for the visited paths appear under `pages` (eg `/docs/design/counter`).
   - both client kinds are present under `client kinds`: `Web` (the http/browser visits) and `Terminal` (the ssh session).
   - for dev/prod (geoip enabled in the deploy build), a country appears under `countries` for the web visits. The geoip database (`assets/databases/country.mmdb`, ~8MB) is gitignored and workspace-owned, so it reaches the app bucket by being copied into `site/assets` by the deploy's `<DirCopy/>` step: run `just beet-shared pull` before deploying, and if `countries` is empty verify `country.mmdb` is present under `s3://beet-site--<stage>--app/assets/databases/`. The lookup is switched on by the `GeoIpDb` spread on the serving `<Router>` in `site/main.bsx`, which reads that path out of the app bucket like any other asset; without it every country is empty by authorship, and a declared-but-absent database logs `geoip: no country database at ..` on boot.

REGRESSION GUARD (fixed): the deploy created `beet-site--<stage>--analytics` from the stack's app name while the *running binary* derived the same name independently, so a missing app name made it write to `beet--<stage>--analytics`, a table that does not exist. Every event failed with a DynamoDB `ResourceNotFoundException` logged to `/var/log/beet-site.log` while the site served perfectly, so a fully green a-d reported `0 events` on check `e`. The name is no longer derived twice: `site/main.bsx` declares the table once and both sides compose it through the one resolved `Stack` (pinned by `one_declaration_names_the_table_for_both_sides` in `crates/beet_extra/src/infra/templates.rs`). A store that cannot answer now also raises ONE loud error naming the table and region on its first failed write and then poisons itself (`AnalyticsStore::record`), rather than a silent error per event. If a summary reports `0 events`, look for that error in the box log first.

### f. the nightly rollup (archive, aggregate, expire)

The analytics table does not grow forever. A `<ScheduledJobBlock label="rollup-daily"/>` invokes the `rollup` lambda at 03:00 UTC, which boots the entry's `jobs` verb out of the app bucket and dispatches `rollup` into its router. Each run, in this order and no other: archives every complete day not yet covered as `analytics/raw/<date>.ndjson.gz` in `beet-site--<stage>--runtime-ops`, writes that day's `AnalyticsRollup` rows into `beet-site--<stage>--analytics-rollup`, reads both back, and only then stamps the day's raw rows with the `ttl` DynamoDB expires them by (30 days for requests, 90 for the client-reported streams). The aggregates and the archive are forever; the raws are not.

- `beet analytics summary` now reports over BOTH stores: aggregates for the days they cover, raws for the recent window they do not. `--raw-only` skips the aggregates, `--rollup <name>` overrides the aggregate store (it defaults to the events store plus `-rollup`).
- The lambda publishes no url, no gateway and no hostname (`LambdaBlock`'s `http=false`), so the only way to run it by hand is `aws lambda invoke --function-name beet-site--<stage>--rollup-function --payload '<the json below>' out.json --cli-binary-format raw-in-base64-out` (the payload is a `ScheduledInvoke`; anything else fails to deserialize and fails the invocation, deliberately). Its logs are `/aws/lambda/beet-site--<stage>--rollup-function`. Note ONE dash before `function`: the label is `rollup` and `LambdaBlock` composes `<app>--<stage>--<label>-function`, so the doubled form is a name that does not exist.
- The backfill of a history recorded before the pipeline existed is just the FIRST ordinary run: it covers every uncovered day, and re-invoking continues where a timeout stopped (stamping runs last and only for the days that run covered, so an interrupted sweep leaves days archived-but-unaggregated, never raws expiring un-archived). A row already past its window takes a two-day grace floor rather than an expiry in the past, so a botched sweep is readable the next morning. `--full` (`{"kind":"beet.scheduled_invoke.v1","method":"Post","path":"rollup?full=true"}`) re-derives days already covered, for a new aggregate schema; it is idempotent, since aggregate ids and archive object names are pure functions of the day.
- WATCH FOR: a report reading `0 days` with a nonzero scan means nothing was covered, which is correct only when every complete day is already archived AND aggregated. A report that archived nothing and stamped rows is the failure the ordering exists to prevent and cannot happen without the code changing; treat it as a bug, not an operational hiccup.
- The `ttl` attribute is an IN-PLACE update to the live table (pinned by `enabling_ttl_is_an_in_place_change`). Confirm the prod plan shows the analytics table as an update, never a replacement.

The local query takes ~50s and prints server log lines to stdout ahead of the summary; `tail` it. Cross-check a remote total against `aws dynamodb scan --table-name beet-site--<stage>--analytics --select COUNT` before reporting apparent data loss: `DynamoStore::list` used to issue one unpaginated `scan`, so it truncated silently at the first 1MB page (~4k rows) and reported that page as the total, and the per-row read fanned out unbounded so connect timeouts were miscounted as unreadable rows. Both are fixed (`dynamo_store.rs` paginates, `BlobStore::GET_ALL_CONCURRENCY` bounds the fan-out), but a wildly varying total across runs is the signature. An automated in-process version of the web half of this flow (http request -> request event, beacon -> page view, prior events retained, the beacon endpoint skipped) lives in `tests/beet_site_analytics.rs`; run it with `cargo test --test beet_site_analytics --features "router,json,fs,testing"` for a fast pre-deploy check of the wiring. The terminal page-view path is unit-tested in `beet_router/src/navigate/navigator.rs`.

## Step 1: Local

```sh
cargo run -p beet-cli -- --main=site serve --server=http,ssh    # run in background
```

Read the bound http + ssh ports from the serve output (defaults 8337 / 8339). Run the full verification (a-e; check `f` is cloud-only, its lambda has no local counterpart) against `http://localhost:<http_port>` and `127.0.0.1:<ssh_port>`. This step is also the shakedown: run the browser test and settle the ssh driver here, recording any changes above. Kill the server when done. No cloud or DNS impact.

## Step 2: Dev

```sh
just beet-validate            # resolves, no cloud
just beet-plan                # EYEBALL: dev must touch only beet-site--dev--* and dev.beet.org
just beet-deploy              # build -> stores -> sync site/ + assets/ -> replace box -> watch -> purge
```

Run the full verification (a-f) against `https://dev.beet.org` (ssh on `app.dev.beet.org` port 22), allowing several minutes for cloud-init + Let's Encrypt to settle (retry with a sane budget; a `521` means the origin is not serving yet). Then ALWAYS tear down:

```sh
just beet-destroy             # removes the dev stack
```

Confirm teardown: no dev Lightsail instance or static IP (`aws lightsail get-instances`, `aws lightsail get-static-ips`), no `beet-site--dev--*` S3 buckets (`--app`, `--runtime-ops` and the on-demand `--artifacts`), no dev analytics tables (`--analytics` and `--analytics-rollup`, `aws dynamodb list-tables`), no `beet-site--dev--rollup-daily` schedule (`aws scheduler list-schedules`) and no `beet-site--dev--rollup-function` lambda (ONE dash before `function`, as above: the doubled form is a name that never exists, so checking it always passes), and no `dev.beet.org` / `app.dev.beet.org` Cloudflare records. A clean teardown leaves ONLY `beet-site--prod--*`, `beet-site--shared--assets`, `beet--shared--assets` and `beet-state`. Dev is intentionally not left running.

## Step 3: Prod

```sh
just beet-validate --stage=prod
just beet-plan --stage=prod   # EYEBALL: prod creates beet.org + www.beet.org at the prod static IP
just beet-deploy --stage=prod
```

EYEBALL the plan for three things specifically: that `aws_s3_bucket.beet_site__prod__app` and `aws_dynamodb_table.beet_site__prod__analytics` appear only as *refreshes* or in-place updates, never destroys (they carry the site content and the whole analytics history, and enabling `ttl` must be an update); that the new resources (the `analytics-rollup` table, the `runtime-ops` bucket, the `rollup` lambda and the `rollup-daily` schedule) are additions; and that the DNS changes are the ones you expect.

A BASELINE REDEPLOY OF UNCHANGED MAIN PLANS EXACTLY ONE CHANGE. Prod already holds every resource, so a code-only deploy reads `Plan: 0 to add, 1 to change, 0 to destroy`: the rollup lambda's `s3_key`, `source_code_hash` and its two `BEET_DEPLOY_*` variables. Everything else, the app bucket, both analytics tables, the runtime-ops bucket, the instance and all three Cloudflare records, appears as a `Refreshing state...` line and nothing more. The box itself is NOT in the plan at all: the binary reaches it through the release pointer, so `uptime` on the box spans previous deploys and an instance uptime that resets on a code-only deploy means `user_data` moved.

Run the full verification (a-f) against `https://beet.org` (and confirm `https://www.beet.org` serves too; ssh on `app.beet.org` port 22). LEAVE PROD UP. Teardown, if ever needed, is `just beet-destroy --stage=prod`.

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

1. **Hydrate both asset trees** (`just beet-shared pull` for `./assets`, `just site-shared pull` for `./site/assets`). The site sync mirrors `site/` with `delete=true`, so it publishes whatever the checkout holds. `SyncS3Bucket::assert_mirrorable` refuses a missing or empty source rather than emptying the bucket, but a *stale* tree syncs happily and silently ships old assets.
2. **The apply runs once per layer, and the order matters.** `<TofuApply layer="storage"/>` brings up the stores (the addresses blocks declare under the `storage` layer, an overridable per-block field), the `<DirSync/>` fills them, then a bare `<TofuApply/>` converges the whole stack and replaces the box against content that is already published. A single apply built the instance first, and it booted with `no entry document found in the --store backend`. Naming a layer no block declares is a loud error, never a silent skip. Since the box is replaced rather than rolled, there is no health gate and no rollback: `LightsailWatch` is a CloudWatch log tail, not a readiness check, so re-check the site after the deploy returns, not during.
3. **Diff the rendered configs before/after** whenever `beet_infra` or the deploy render changes: `.agents/skills/deploy/render_all.sh before` on the clean tree FIRST, then after the change `.agents/skills/deploy/render_all.sh after .agents/tmp/render/before` renders all eight stacks and diffs each against the baseline. `HASH-ONLY` means only `source_code_hash` moved, ie a rebuilt binary shipping as a code-only deploy, not a config change. Any other diff must be deliberate and explainable line by line (invariants: no existing physical resource name changes, no replacement of an existing resource). The dumps are disposable and live in `.agents/tmp/`; nothing is committed.
4. Never run with `--stage=prod` except in Step 3.
