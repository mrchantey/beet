#	temporarily using just to work with beet
# Eventually all these patterns should be rolled into the cli
#
# ```rust
# cargo binstall just
# just --list
# just test-all
# ```
#
set dotenv-load := true

# fresh compile of beet is so big it keeps asking for bigger stacks.. this is 1GB 😭

export RUST_MIN_STACK := '1073741824'

# export RUST_MIN_STACK := '268435456'
# min-stack := 'RUST_MIN_STACK=134217728'
# min-stack := 'RUST_MIN_STACK=67108864'
# min-stack := 'RUST_MIN_STACK=33554432'

test-threads := '--test-threads=8'

default:
	just --list --unsorted

#💡 Init

# Install the cli. The dev commands (run-wasm, check, export-static, s3-sync, ...)
# are wired in the repo's `main.bsx`, discovered at runtime — no scene to load.
init-cli:
	just install-cli

# Set up a fresh checkout: cli, both asset trees, and the ml default model.
# The two trees are separate sources of record: the workspace's own
# (`beet--shared--assets`, what the examples/tests/wasm builds read) and the
# site's (`beet-site--shared--assets`, what the website serves). Neither derives
# from the other, so a fresh checkout pulls both.
init-repo:
	just init-cli
	just beet-shared pull
	just site-shared pull
	mkdir -p crates/beet_ml/assets/ml && cp ./assets/ml/default-bert.ron crates/beet_ml/assets/ml/default.bert.ron

#💡 CLI

# Run a beet cli command (scene/site/server) with no install step, eg
#   just beet --main=examples/spatial/seek_3d.bsx
# `--features winit` links winit/wgpu + the example scene templates; the binary
# resolves the assets dir from the workspace root itself (see `winit_default_plugins`),
# so no `BEVY_ASSET_ROOT` is needed. Add `,ml` to run an ml scene (eg `fetch.bsx`).
# Headless verification: prefix BEET_SCREENSHOT=/tmp/x.png BEET_SCREENSHOT_FRAME=N to
# capture a frame to a PNG and exit (see `crates/beet-cli/src/render.rs`).
beet *args:
  cargo run -p beet-cli -- {{ args }}

# Deploy the beet website to its AWS Lightsail box; --stage=prod targets prod
# (default dev). Lean headless build (no winit/ml) and AWS_PROFILE cleared so
# tofu/aws/s3 use the explicit `.env` keys rather than a global profile.
# `--main=site`: the SITE entry declares its own resources and deploy verbs, so
# the application that runs on them is the thing that provisions them.
# `infra,extra` links the deploy host and the IaC verb routes; without them the
# site entry's `bx:features` gate skips the whole subtree and the verb is absent.
beet-deploy *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- --main=site deploy {{ args }}
# Re-publish the site to S3 without a redeploy (site assets: `site-shared push`).
beet-sync *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- --main=site sync {{ args }}
# Tail the deployed instance's logs.
beet-watch *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- --main=site watch {{ args }}
# Refresh the files site/assets borrows from the workspace tree (the wasm binary,
# the geoip database, the robot faces). Runs ahead of every publish anyway.
beet-assets *args:
  cargo run -p beet-cli --features infra,extra -- --main=site assets {{ args }}
# Tear the deployed stack down (pass --stage=prod for the prod stack). Stage only:
# the `shared` stage (the assets bucket) has its own verbs under `site-shared`.
beet-destroy *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- --main=site destroy --force {{ args }}
# Resolve the deploy config without touching cloud (safe pre-apply check).
beet-validate *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- --main=site validate {{ args }}
# Show the tofu plan without applying (eyeball before deploy).
beet-plan *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- --main=site plan {{ args }}
# The WORKSPACE assets bucket (`beet--shared--assets`), the source of record for
# ./assets: `just beet-shared plan|apply|pull|push|..`. Rooted at the workspace
# entry, since these assets belong to the repo rather than to the website.
beet-shared *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- shared {{ args }}
# The SITE assets bucket (`beet-site--shared--assets`), the source of record for
# ./site/assets: `just site-shared plan|apply|pull|push|..`.
site-shared *args:
  AWS_PROFILE= cargo run -p beet-cli --features infra,extra -- --main=site shared {{ args }}

# Build beet-cli in release into the real ./target (full incremental caching) and
# symlink the binary into the cargo bin dir. This is far faster than `cargo install`,
# which rebuilds from scratch in a throwaway target every time. Iterating? Just re-run
# `cargo build --release -p beet-cli` on its own — the symlink points at
# target/release/beet, so the fresh binary is picked up with no reinstall step.
install-cli *args:
  #!/usr/bin/env bash
  set -euo pipefail
  cargo build --release -p beet-cli --all-features {{ args }}
  bin="$(realpath "${CARGO_TARGET_DIR:-target}/release/beet")"
  dst="${CARGO_HOME:-$HOME/.cargo}/bin/beet"
  ln -sf "$bin" "$dst"
  echo "linked $dst -> $bin"

#💡 Aliases

fmt *args:
	rustup default nightly
	cargo fmt {{ args }}
	rustup default stable

#💡 Test

test-all *args:
	@if [ ! -d assets ] || [ -z "$(ls -A assets 2>/dev/null)" ]; then \
		echo "please download the workspace assets: just beet-shared pull"; \
		exit 1; \
	fi
	@if [ ! -d site/assets ] || [ -z "$(ls -A site/assets 2>/dev/null)" ]; then \
		echo "please download the site assets: just site-shared pull"; \
		exit 1; \
	fi
	just test-core {{ args }}
	just test-scripting-fallback {{ args }}
	# `bevy_default`-enabling crates each run in their own cargo invocation —
	# unifying `bevy/default` across the whole graph has tripped a mold linker bug.
	# `|| exit 1` on every loop: a `for` loop exits with the status of its *last*
	# iteration, so without it a failing package in the middle is silently passed over.
	for pkg in {{ _extra-pkgs }}; do just _test-pkgs "$pkg" {{ args }} || exit 1; done
	for pkg in {{ _extra-pkgs-wasm }}; do just _test-pkgs-wasm "$pkg" {{ args }} || exit 1; done
	just test-rsx {{ args }}
	# beet-cli is not in `_core-pkgs`: it is the binary crate, so `_core-features`'
	# enumerate-everything approach would co-enable mutually exclusive target
	# features (`web`/`cloudflare` alongside the native stack). `--all-features` is
	# safe here because its wasm-only deps are already target-gated.
	cargo test -p beet-cli --all-features {{ args }} -- {{ test-threads }}
	just test-facade-doc {{ args }}

# `--all-features` is never safe workspace-wide: it enables `cuda`, whose
# `cudarc` build script cannot build on every host. Both recipes below therefore
# run the same crate sets `test-all` does, through `_core-features`.

test-all-lib *args:
	just _test-pkgs "{{ _core-pkgs }}" --lib {{ args }}
	for pkg in {{ _extra-pkgs }}; do just _test-pkgs "$pkg" --lib {{ args }} || exit 1; done

# The doc-only pass. `_test-pkgs` already runs each crate's doctests as part of
# `cargo test`, so `test-all` covers these; this is the quick pass when only a
# doc comment changed.
test-all-doc *args:
	just _test-pkgs "{{ _core-pkgs }}" --doc {{ args }}
	for pkg in {{ _extra-pkgs }}; do just _test-pkgs "$pkg" --doc {{ args }} || exit 1; done
	just test-facade-doc {{ args }}

# The `beet` facade's doctests, ie its README (`#![doc = include_str!]`). Like
# beet-cli it is an aggregate crate, so `_core-features`' enumerate-everything
# would co-enable mutually exclusive targets (`web`/`cloudflare`/`embedded`);
# instead run the default surface, then the behaviour surface the README's
# example needs (it is `#[cfg(feature = "action")]`-gated, so the default pass
# only compiles it away).
test-facade-doc *args:
	cargo test -p beet --doc {{ args }} -- {{ test-threads }}
	cargo test -p beet --doc --features action {{ args }} -- {{ test-threads }}

# rsx_site (the typed-authoring example) is excluded from the `test-core` /
# `test-all` package lists (its `src/codegen` route modules are generated, not
# committed). Regenerate them, then run its render + tui tests (`--features tui`
# enables the tui-gated `tui.rs` while keeping the default `web` target).
test-rsx *args:
	cargo run -p rsx_site --no-default-features --features codegen
	cargo test -p rsx_site --features tui {{ args }}

# client for the ssh_server example
# the constant debug host key means fingerprints are stable between restarts
ssh-client:
	ssh -p 8339 127.0.0.1

snap:
	cargo test -p beet_core 				--lib --all-features -- --snap
	cargo test -p beet_core_macros 	--lib --all-features -- --snap
	cargo test -p beet_net					--lib --features=server,ureq,tungstenite,native-tls -- --snap
	cargo test -p beet_router 			--lib --all-features -- --snap

# The libtest path (`custom_test_frameworks`) and the `nightly` feature are
# nightly-only. On nightly we use `--all-features`; on stable we enable every
# feature *except* `nightly` / `custom_test_frameworks` so the stable
# `inventory` runner is exercised. Validate the libtest path explicitly with:
#   cargo +nightly test -p beet_core --test test_test --features custom_test_frameworks

# Native test crate sets.
_core-pkgs := "beet_core_shared beet_core_macros beet_async beet_core beet_infra beet_net beet_ui beet_router beet_thread beet_action"

# Wasm test crate sets. beet_infra runs its config/definition tests under deno
# like everywhere else: its work-dir test helper has a wasm arm, and only the two
# `Project::validate` tests (the native tofu cli) are native-gated.
_core-pkgs-wasm := "beet_core beet_infra beet_net beet_ui beet_router beet_thread beet_action"

# Crates that enable `bevy_default` — each runs in its own cargo invocation
# in `test-all` (see comment there). Excluded from `test-core`.
_extra-pkgs := "beet_spatial beet_ml beet_extra"

# Subset of `_extra-pkgs` whose wasm tests pass. beet_ml runs under deno, whose
# WebGPU grants a real wgpu device: the bert forward pass initializes it through
# burn's async setup (`default_device_async`) and probes first, skipping with a
# warning on a host that grants no adapter (gpu-less CI).
_extra-pkgs-wasm := "beet_spatial beet_ml beet_extra"

# Computes the cargo feature flag for the in-scope crates by enumerating each
# crate's `[features]` and excluding the ones that must not be co-enabled.
# Always excludes:
# - `default`: redundant, cargo keeps default features on without naming them
# - `ndarray` / `cuda`: burn backends mutually exclusive with `wgpu` (the
#   default). Co-enabling them links conflicting backend runtimes and corrupts
#   the heap at process teardown, so `--all-features` is never safe here.
# On stable additionally excludes `nightly` / `custom_test_frameworks`, the
# nightly-only test-runner features that stable cannot compile.
# `extra` is a `|`-joined list of additional feature names to exclude; the wasm
# runner passes `cloudflare` (see `_test-pkgs-wasm`).
_core-features pkgs extra="":
	#!/usr/bin/env bash
	set -euo pipefail
	base='default|ndarray|cuda'
	if ! rustc --version | grep -q nightly; then
		base="nightly|custom_test_frameworks|$base"
	fi
	[ -n "{{ extra }}" ] && base="$base|{{ extra }}"
	exclude="/($base)\$"
	feats=$(for c in {{ pkgs }}; do
		# Crates may be nested (e.g. crates/beet_core/macros) — resolve by package name.
		toml=$(grep -lE "^name *= *\"$c\"$" crates/$c/Cargo.toml crates/*/*/Cargo.toml 2>/dev/null | head -1)
		# Match only `name = ...` feature lines, skipping comments (`#`) that may
		# themselves contain an `=`.
		awk -v C=$c '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{key=$0; sub(/[[:space:]]*=.*/,"",key); print C"/"key}' "$toml"
	done | grep -vE "$exclude" | paste -sd, -)
	echo "--features $feats"

# Shared native cargo test runner over a space-separated list of crates.
_test-pkgs pkgs *args:
	#!/usr/bin/env bash
	set -euo pipefail
	feats=$(just _core-features "{{ pkgs }}")
	crates=$(printf -- "-p %s " {{ pkgs }})
	cargo test $crates $feats {{ args }} -- {{ test-threads }}

# Shared wasm cargo test runner over a space-separated list of crates.
# Excludes `cloudflare`: it pulls the `worker` SDK, whose module init expects the
# Cloudflare Workers runtime and hangs under the Deno wasm test runner. Every
# other feature is enabled, `testing_embedded` included — its `linkme` slice is
# cfg'd out on wasm, so the feature is inert there rather than unbuildable.
_test-pkgs-wasm pkgs *args:
	#!/usr/bin/env bash
	set -euo pipefail
	feats=$(just _core-features "{{ pkgs }}" "cloudflare")
	crates=$(printf -- "-p %s " {{ pkgs }})
	cargo test $crates --lib --target wasm32-unknown-unknown $feats {{ args }} -- {{ test-threads }}

test-core *args:
	just _test-pkgs "{{ _core-pkgs }}" {{ args }}
	just _test-pkgs-wasm "{{ _core-pkgs-wasm }}" {{ args }}

test-core-wasm *args:
	just _test-pkgs-wasm "{{ _core-pkgs-wasm }}" {{ args }}

# Run a crate's wasm suite inside a headless browser instead of deno: only the
# `#[beet_core::test(browser)]` dom tests run there (everything else skips, and
# vice versa under deno). Needs chromedriver + a chromium on PATH. For a
# WebGPU-granting session append `-- --chrome-args='--enable-unsafe-webgpu --use-angle=gl'`.
test-wasm-browser crate *args:
	BEET_WASM_HOST=browser cargo test -p {{ crate }} --lib --target wasm32-unknown-unknown {{ args }}

# Headless-chrome verification of the browser render boot: serves the built
# beet-render.wasm at a generated page and asserts the WebGPU boot claims a
# canvas and draws pixels while the GPU-less boot stays surfaceless. Needs
# `just build-wasm-render` first, plus the test-browser PATH deps.
check-wasm-render *args:
	cargo test -p beet-cli --lib {{ args }} -- --include-ignored --include '*browser_render_boot*'

# The native browser-driving smoketests: the beet_net webdriver suite against
# local fixtures, and beet_ui's reactivity-runtime proof. Same PATH deps.
test-browser *args:
	cargo test -p beet_net --features webdriver,testing --lib {{ args }} -- --include-ignored --include '*webdriver*'
	cargo test -p beet_ui --lib {{ args }} -- --include-ignored --include '*reactivity_in_browser*'

# The `Script` backends are mutually exclusive at compile time, so the host-realm
# fallbacks need their own invocations: `test-core`/`test-core-wasm` enumerate
# every feature, which always selects the embedded `quickjs` engine and compiles
# the fallbacks out. Native exercises the sandboxed deno child, wasm the
# permissionless deno Worker. Both need deno on PATH, the same dependency the
# wasm test runner already imposes.
test-scripting-fallback *args:
	cargo test -p beet_action --lib {{ args }} -- {{ test-threads }}
	cargo test -p beet_action --lib --target wasm32-unknown-unknown {{ args }} -- {{ test-threads }}


# The floor (assets/wasm/beet-min.wasm): the smallest binary that is still a beet
# runtime in the browser, the `web` base and nothing else. A page whose program uses
# only the core vocabulary mounts this. Needs `just install-cli`.
build-wasm-min:
	beet build-wasm --release --package=beet-cli --bin=beet --features=web_min --out=assets/wasm/beet-min.wasm

# The render middle (assets/wasm/beet-render.wasm): the browser binary plus the
# windowed render stack (wgpu via WebGPU, the spatial + example scenes), so a
# render scene `.bsx` boots in a tab without the full binary's weight; see the
# `web_render` comment in crates/beet-cli/Cargo.toml. Needs `just install-cli`.
build-wasm-render:
	beet build-wasm --release --package=beet-cli --bin=beet --features=web_render --out=assets/wasm/beet-render.wasm

# The ceiling (assets/wasm/beet-full.wasm): every feature a browser binary can boot
# with, ie the example surface, the render stack and ml backend, the perceive-act
# head, the thread runtime, the tui host, infra definitions and the embedded
# JavaScript engine; see the `web_full` comment in crates/beet-cli/Cargo.toml.
# Slower to build (the JS engine's wasi sysroot downloads once) and several times
# the artifact, so a page mounts it only when its program needs more than the floor.
# Ends by refreshing the site's borrowed copy, so a rebuilt binary cannot go
# stale in site/assets (the tutorial page serves it from there).
build-wasm-full:
	beet build-wasm --release --package=beet-cli --bin=beet --features=web_full --out=assets/wasm/beet-full.wasm
	beet --main=site assets

# Build and serve the browser-wasm example at http://127.0.0.1:8337. Open the page
# to run a headless beet program (examples/wasm/hello.bsx) in the browser; its
# console output renders on the page via <RenderConsole>. The entry roots at the
# workspace (<StoreRoot>) so the served examples are reachable and --watch
# live-reloads on edit. Its /scripting page mounts the full binary, so run
# `just build-wasm-full` once to serve that page too.
serve-wasm *args:
	just build-wasm-min
	beet --main=examples/wasm --watch {{ args }}

clear-rust-analyzer:
	rm -rf $CARGO_TARGET_DIR/rust-analyzer

clear-ice:
	rm -f rustc-ice-*

clear-artifacts:
	just clear-ice
	rm -rf examples/rsx_site/src/codegen
	rm -rf launch.ron
	rm -rf target

# massive purge
clear-all:
	just clear-artifacts
	just clear-rust-analyzer
	cargo clean
	sccache --stop-server && rm -rf $SCCACHE_DIR
	rm -rf $CARGO_TARGET_DIR

tree:
	cargo tree --depth=2 -e=no-dev

#💡 Misc

patch:
	cargo set-version --bump patch

publish *args:
	cargo publish --workspace --allow-dirty --no-verify {{ args }}

# Cargo search but returns one line
search *args:
	cargo search {{ args }} | head -n 1
