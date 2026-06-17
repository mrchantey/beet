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

# The upstream bucket holding shared assets (models, fonts, fixtures).
assets-bucket := 's3://beet-site--prod--assets'

default:
	just --list --unsorted

#💡 Init

# Install the cli and load its default commands (serve, export-static, s3-sync, run-wasm, ...).
init-cli:
	just install-cli
	cargo run -p beet-cli --bin export_scenes
	beet load target/scenes/default-cli.json

# Set up a fresh checkout: cli, assets, and the ml default model.
init-repo:
	just init-cli
	just pull-assets
	mkdir -p crates/beet_ml/assets/ml && cp ./assets/ml/default-bert.ron crates/beet_ml/assets/ml/default.bert.ron

# Pull shared assets from the upstream bucket into ./assets.
pull-assets:
	beet s3-sync --src={{ assets-bucket }} --dst=./assets --delete

# Push local ./assets up to the upstream bucket.
push-assets:
	beet s3-sync --src=./assets --dst={{ assets-bucket }} --delete

#💡 CLI

# Run a cli command as if it was installed
cli *args:
  beet {{ args }}
  # cargo run -p beet-cli -- {{ args }}

# `--locked` pins the workspace bevy rc, since a bare install re-resolves
# pre-release deps to a newer, incompatible candidate.
install-cli *args:
  cargo install --locked --path crates/beet-cli {{ args }}

#💡 Aliases

fmt *args:
	cargo fmt {{ args }}

#💡 Test

test-all *args:
	@if [ ! -d assets ] || [ -z "$(ls -A assets 2>/dev/null)" ]; then \
		echo "please download assets directory: just pull-assets"; \
		exit 1; \
	fi
	just test-core {{ args }}
	# `bevy_default`-enabling crates each run in their own cargo invocation —
	# unifying `bevy/default` across the whole graph has tripped a mold linker bug.
	for pkg in {{ _extra-pkgs }}; do just _test-pkgs "$pkg" {{ args }}; done
	for pkg in {{ _extra-pkgs-wasm }}; do just _test-pkgs-wasm "$pkg" {{ args }}; done
	just test-rsx {{ args }}
	# beet-cli is currently commented out of the workspace; re-add when restored.
	# cargo test -p beet-cli --all-features {{ args }} -- {{ test-threads }}

# cargo test --workspace -- {{args}}
# cargo test --workspace --all-features -- {{args}}

test-all-lib *args:
	cargo test --workspace 			--lib 	--all-features																	{{ args }} -- {{ test-threads }}

test-all-doc *args:
	cargo test --workspace 			--doc 	--all-features																	{{ args }} -- {{ test-threads }}

test-fmt:
	cargo fmt 				--check

test-ci *args:
	just test-fmt
	just test-rsx

# client for the ssh_server example
# the constant debug host key means fingerprints are stable between restarts
ssh-client:
	ssh -p 8322 127.0.0.1

snap:
	cargo test -p beet_core 				--lib --all-features -- --snap
	cargo test -p beet_core_macros 	--lib --all-features -- --snap
	cargo test -p beet_net					--lib --features=server,ureq,tungstenite,native-tls,flow -- --snap
	cargo test -p beet_build 				--lib --all-features -- --snap
	cargo test -p beet_router 			--lib --all-features -- --snap

# The libtest path (`custom_test_frameworks`) and the `nightly` feature are
# nightly-only. On nightly we use `--all-features`; on stable we enable every
# feature *except* `nightly` / `custom_test_frameworks` so the stable
# `inventory` runner is exercised. Validate the libtest path explicitly with:
#   cargo +nightly test -p beet_core --test test_test --features custom_test_frameworks

# Native test crate sets.
_core-pkgs := "beet_core_shared beet_core_macros beet_async beet_core beet_infra beet_net beet_ui beet_router beet_thread beet_action"

# Wasm test crate sets (skip crates that don't build for wasm).
_core-pkgs-wasm := "beet_core beet_net beet_ui beet_router beet_thread beet_action"

# Crates that enable `bevy_default` — each runs in its own cargo invocation
# in `test-all` (see comment there). Excluded from `test-core`.
_extra-pkgs := "beet_spatial beet_ml"

# Subset of `_extra-pkgs` that builds for wasm (beet_ml doesn't — `getrandom`
# needs the `wasm_js` feature).
_extra-pkgs-wasm := "beet_spatial"

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
# runner passes `testing_embedded`, whose `linkme` distributed slice does not
# compile off bare metal (native keeps it, exercising the linkme declaration).
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
# Excludes `testing_embedded`: its `linkme` distributed slice is unsupported on
# wasm32 (the embedded runner is bare-metal only).
_test-pkgs-wasm pkgs *args:
	#!/usr/bin/env bash
	set -euo pipefail
	feats=$(just _core-features "{{ pkgs }}" "testing_embedded")
	crates=$(printf -- "-p %s " {{ pkgs }})
	cargo test $crates --lib --target wasm32-unknown-unknown $feats {{ args }} -- {{ test-threads }}

test-core *args:
	just _test-pkgs "{{ _core-pkgs }}" {{ args }}
	just _test-pkgs-wasm "{{ _core-pkgs-wasm }}" {{ args }}

test-core-wasm *args:
	just _test-pkgs-wasm "{{ _core-pkgs-wasm }}" {{ args }}


test crate *args:
	just watch cargo test -p {{ crate }} --lib -- --watch=true {{ args }}

test-int crate test *args:
	just watch cargo test -p {{ crate }} --test {{ test }} {{ args }}

test-e2e crate *args:
	just watch cargo test -p {{ crate }} --lib --features=e2e -- 														--e2e	--watch {{ args }}

test-doc crate *args:
	just watch cargo test -p {{ crate }} --doc 																														{{ args }}

test-wasm crate *args:
	just watch cargo test -p {{ crate }} --lib --target wasm32-unknown-unknown -- 								--watch {{ args }}

test-wasm-feat crate *args:
	just watch cargo test -p {{ crate }} --lib --target wasm32-unknown-unknown --all-features -- 					{{ args }}

test-wasm-e2e crate test_name *args:
	just watch cargo test -p {{ crate }} --test {{ test_name }} --target wasm32-unknown-unknown -- 	--watch {{ args }}

test-clanker:
	just cli clanker 										\
	--oneshot --image										\
	-f=assets/tests/agents/prompt.txt		\
	--out-dir=assets/tests/agents/out

example-chat *args:
	just watch cargo run --example chat 	--features=native-tls,thread -- {{ args }}

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
