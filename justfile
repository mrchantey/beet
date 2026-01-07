#	I'm temporarily using just to work with beet
# Eventually all these patterns should be rolled into t
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

# Pull assets into their respective crates.
init-repo:
    just pull-assets
    mkdir -p crates/beet_ml/assets/ml && cp ./assets/ml/default-bert.ron crates/beet_ml/assets/ml/default.bert.ron
    just install-sweet
    cargo launch codegen

pull-assets:
    cargo launch --only=pull-assets

push-assets:
    cargo launch --only=push-assets

# just test-site
# just export-scenes
#💡 CLI

# Run a cli command as if it was installed
cli *args:
    cargo run -p beet-cli -- {{ args }}

install-cli *args:
    cargo install --path crates/beet-cli {{ args }}

lambda-build:
    cargo lambda build -p beet_site --features beet/lambda --release --lambda-dir target/lambda/crates

run-mod *args:
    just sweet mod --exclude */codegen/* {{ args }}

# Run and watch a workspace example
run example *args:
    just watch just run-ci {{ example }} {{ args }}

run-feat example *args:
    just run {{ example }} --all-features {{ args }}

# Run an example without watching
run-ci example *args:
    cargo run --example {{ example }} {{ args }}

# Run and watch a crate example
run-p crate example *args:
    just watch cargo run -p {{ crate }} --example {{ example }} {{ args }}

# Run and watch a crate build step
run-b crate *args:
    just watch cargo run -p {{ crate }} --bin run-build --features=build {{ args }}

run-csr:
    cargo run --example csr --features=client
    just watch just build-csr

build-csr:
    cargo build --example csr --features=client --target wasm32-unknown-unknown
    wasm-bindgen --out-dir target/examples/csr/wasm --out-name main --target web --no-typescript $CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/examples/csr.wasm
    sweet serve target/examples/csr

build-todo-app *args:
    cd examples/todo-app && cargo launch
    cd examples/todo-app && cargo build 	--no-default-features --features=client --target wasm32-unknown-unknown
    cd examples/todo-app && wasm-bindgen --out-dir target/client/wasm --out-name main --target web --no-typescript $CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/todo-app.wasm

launch *args:
    cargo launch -w {{ args }}

todo-app *args:
    cd examples/todo-app && cargo launch -- --watch {{ args }}

run-hydration:
    just watch just build-hydration

run-ssr:
    just watch cargo run --example ssr --features=server

build-hydration:
    cargo run --example hydration --features=css
    cargo build --example hydration --target-dir=target --features=rsx --target wasm32-unknown-unknown
    wasm-bindgen --out-dir target/examples/hydration/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/hydration.wasm
    sweet serve target/examples/hydration

doc crate *args:
    just watch cargo doc -p {{ crate }} --open {{ args }}

fmt *args:
    cargo fmt {{ args }} && just leptosfmt {{ args }}

# soo bad
leptosfmt *args:
    leptosfmt -q											\
    crates/beet_rsx/**/*.rs 					\
    crates/beet_rsx/**/**/*.rs 				\
    crates/beet_rsx/**/**/**/*.rs 		\
    crates/beet_design/**/*.rs 				\
    crates/beet_design/**/**/*.rs 		\
    crates/beet_design/**/**/**/*.rs 	\
    crates/beet_site/**/*.rs 					\
    crates/beet_site/**/**/*.rs 			\
    crates/beet_site/**/**/**/*.rs 		\
    {{ args }}

#💡 e2e examples

# Run bevy reactive example on an endless loop, it exits on recompile required
run-bevy-rsx:
    while true; do cargo run --example bevy_rsx --features=bevy_default; done

# run-bevy-rsx but stop if there's an error
run-bevy-rsx-if-ok:
    while cargo run --example bevy_rsx --features=bevy_default && [ $? -eq 0 ]; do :; done

# just cli watch -p beet_site {{args}}
build-site *args:
    just cli build -p beet_site {{ args }}

run-site *args:
    just cli run -p beet_site {{ args }}

deploy-site *args:
    just cli deploy -p beet_site --release

#💡 Test

test-all:
    @if [ ! -d assets ] || [ -z "$(ls -A assets 2>/dev/null)" ]; then \
    	echo "please download assets directory: just pull-assets"; \
    	exit 1; \
    fi
    just test-core
    just test-flow
    just test-rsx
    cargo test -p sweet-cli --all-features -- {{ test-threads }}
    cargo test -p beet-cli  --all-features -- {{ test-threads }}

# cargo test --workspace -- {{args}}
# cargo test --workspace --all-features -- {{args}}

test-all-lib *args:
    cargo test --workspace 			--lib 	--all-features																	{{ args }} -- {{ test-threads }}

test-all-doc *args:
    cargo test --workspace 			--doc 	--all-features																	{{ args }} -- {{ test-threads }}

test-fmt:
    cargo fmt 				--check
    just leptosfmt 		--check

test-ci *args:
    just test-fmt
    just test-rsx

snap:
    cargo test -p beet_core 				--lib --all-features -- --snap
    cargo test -p sweet 						--lib --all-features -- --snap
    cargo test -p beet_core_macros 	--lib --all-features -- --snap
    cargo test -p beet_net					--lib --features=_sweet_runner,reqwest,tungstenite,native-tls -- --snap
    cargo test -p beet_build 				--lib --all-features -- --snap
    cargo test -p beet_design 			--lib --all-features -- --snap
    cargo test -p beet_parse 				--lib --all-features -- --snap
    cargo test -p beet_router 			--lib --all-features -- --snap
    cargo test -p beet_rsx 					--lib --all-features -- --snap
    cargo test -p beet_rsx 					--test css 		--all-features -- --snap
    cargo test -p beet_rsx 					--test props 	--all-features -- --snap

# cargo test -p sweet 			--lib 	--all-features  										 			{{args}} -- {{test-threads}} --e2e
test-core *args:
    cargo test -p beet_core 							--all-features 													 	{{ args }} -- {{ test-threads }}
    cargo test -p beet_core --lib --target wasm32-unknown-unknown  --all-features   {{ args }} -- {{ test-threads }}
    cargo test -p sweet 									 													 								{{ args }} -- {{ test-threads }}
    cargo test -p sweet --lib --target wasm32-unknown-unknown  --all-features   		{{ args }} -- {{ test-threads }}
    cargo test -p beet_core_macros 				--all-features 													 	{{ args }} -- {{ test-threads }}
    cargo test -p beet_net						 	--features=_sweet_runner,reqwest,tungstenite,native-tls  	{{ args }} -- {{ test-threads }}
    cargo test -p beet_net 	--lib --target wasm32-unknown-unknown	 --all-features 	{{ args }} -- {{ test-threads }}

test-flow *args:
    cargo test -p beet_flow 		--all-features 																						{{ args }} -- {{ test-threads }}
    cargo test -p beet_sim		 	--lib																											{{ args }} -- {{ test-threads }}
    cargo test -p beet_spatial																														{{ args }} -- {{ test-threads }}
    cargo test -p beet_flow 		--lib 										--target wasm32-unknown-unknown {{ args }} -- {{ test-threads }}
    cargo test -p beet_spatial 	--lib 									 	--target wasm32-unknown-unknown {{ args }} -- {{ test-threads }}

test-rsx *args:
    cargo test -p beet_dom						 	--features=tokens  																	{{ args }} -- {{ test-threads }}
    cargo test -p beet_dom 	--lib 			--target wasm32-unknown-unknown											{{ args }} -- {{ test-threads }}
    cargo test -p beet_rsx_combinator 	--all-features																			{{ args }} -- {{ test-threads }}
    cargo test -p beet_parse 						--all-features 	 	 																	{{ args }} -- {{ test-threads }}
    cargo test -p beet_rsx_macros 			--all-features 	 	 																	{{ args }} -- {{ test-threads }}
    cargo test -p beet_rsx   						--all-features   																		{{ args }} -- {{ test-threads }}
    cargo test -p beet_rsx 	--lib 			--target wasm32-unknown-unknown 										{{ args }} -- {{ test-threads }}
    cargo test -p beet_router						--features=tokens,server														{{ args }} -- {{ test-threads }}
    cargo test -p beet_router						--lib --features=tokens	--target wasm32-unknown-unknown	 	{{ args }} -- {{ test-threads }}
    cargo test -p beet_build 						--all-features																			{{ args }} -- {{ test-threads }}
    cargo test -p beet_design 					--all-features																			{{ args }} -- {{ test-threads }}
    cargo test -p beet_site							--no-default-features --features=server 						{{ args }} -- {{ test-threads }}

test crate *args:
    sweet test -p {{ crate }} --lib --watch {{ args }}

test-int crate test *args:
    sweet test -p {{ crate }} --watch --test {{ test }} {{ args }}

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

test-rsx-macro *args:
    just watch cargo test -p beet_rsx --test rsx_macro --features=css -- 												--watch {{ args }}

test-agent:
    just cli agent 											\
    --oneshot --image										\
    -f=assets/tests/agents/prompt.txt		\
    --out-dir=assets/tests/agents/out

example-chat *args:
    sweet run -w --example chat 	--features=native-tls,agent -- {{ args }}

example-image *args:
    sweet run -w --example image 	--features=native-tls,agent -- {{ args }}

clear-rust-analyzer:
    rm -rf $CARGO_TARGET_DIR/rust-analyzer

clear-ice:
    rm -f rustc-ice-*

clear-artifacts:
    just clear-ice
    rm -rf crates/beet_design/src/codegen
    rm -rf crates/beet_site/src/codegen
    rm -rf examples/todo-app/src/codegen
    rm -rf examples/todo-app/Cargo.lock
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

expand crate test *args:
    just watch 'cargo expand -p {{ crate }} --test {{ test }} {{ args }}'

patch:
    cargo set-version --bump patch

publish *args:
    cargo publish --workspace --allow-dirty --no-verify {{ args }}

watch *command:
    sweet watch \
    --include '**/*.rs' \
    --exclude '{.git,target,html}/**' \
    --exclude '*/codegen/*' \
    --cmd "{{ command }}"

#💡 Misc

# Cargo search but returns one line
search *args:
    cargo search {{ args }} | head -n 1

# Run a command with the sweet cli without installing it
sweet *args:
    cargo run -p sweet-cli -- {{ args }}

# Install the sweet cli
install-sweet *args:
    cargo install --path crates/sweet/cli {{ args }}
