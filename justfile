# Beet uses the Just command runner
# 
# ```rust 
# cargo binstall just
# just --list
# just test-ci
# ```
#

#💡 Init

set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_spatial beet_flow'
# max cargo build jobs

default:
	just --list --unsorted

# Initialize the repository, pulling assets into their respective crates
init-repo:
	just assets-pull
	mkdir -p crates/beet_ml/assets/ml && cp ./assets/ml/default-bert.ron crates/beet_ml/assets/ml/default.bert.ron
	mkdir -p crates/beet_rsx/assets/fonts && cp ./assets/fonts/* crates/beet_rsx/assets/fonts
	just codegen
# just test-site
# just export-scenes

#💡 CLI

# Run a cli command as if it was installed
cli *args:
	cargo run -p beet-cli -- {{args}}


install-cli *args:
	cargo install --path crates/beet-cli {{args}}


# Run and watch a workspace example
run example *args:
	just watch just run-ci {{example}} {{args}}

run-feat example *args:
	just run {{example}} --all-features {{args}} 

# Run an example without watching
run-ci example *args:
	cargo run --example {{example}} {{args}}

# Run and watch a crate example
run-p crate example *args:
	just watch cargo run -p {{crate}} --example {{example}} {{args}}
# Run and watch a crate build step
run-b crate *args:
	just watch cargo run -p {{crate}} --bin run-build --features=build {{args}}


doc crate *args:
	just watch cargo doc -p {{crate}} --open {{args}}

fmt *args:
	cargo fmt {{args}} && just leptosfmt {{args}}

# soo bad
leptosfmt *args:
	leptosfmt -q											\
	crates/beet_rsx/**/*.rs 					\
	crates/beet_rsx/**/**/*.rs 				\
	crates/beet_rsx/**/**/**/*.rs 		\
	crates/beet_design/**/*.rs 				\
	crates/beet_design/**/**/*.rs 		\
	crates/beet_design/**/**/**/*.rs 	\
	crates/beet_router/**/*.rs 				\
	crates/beet_router/**/**/*.rs 		\
	crates/beet_router/**/**/**/*.rs 	\
	crates/beet_site/**/*.rs 					\
	crates/beet_site/**/**/*.rs 			\
	crates/beet_site/**/**/**/*.rs 		\
	{{args}}

#💡 e2e examples

# Run bevy reactive example on an endless loop, it exits on recompile required
run-bevy-rsx:
	while true; do cargo run --example bevy_rsx --features=bevy_default; done
# run-bevy-rsx but stop if there's an error
run-bevy-rsx-if-ok:
	while cargo run --example bevy_rsx --features=bevy_default && [ $? -eq 0 ]; do :; done

# rm -rf ./target
# mkdir -p ./target/wasm-example
# --html-dir 	target/wasm-example 							\

run-dom-rsx:
	just cli watch 																\
	--package beet 																\
	--example dom_rsx 														\
	--templates-root-dir examples/rsx/dom_rsx.rs 	\
	--wasm 																				\
	--static																			\

# | just watch 'just build-wasm beet dom_rsx'
# sweet serve ./target/wasm-example | \

run-test-site:
	cargo run -p beet_router --example collect_routes
	cargo run -p beet_router --example templates
	cargo run -p beet_router --example html
	sweet serve target/test_site
# --templates-root-dir crates \

run-site *args:
	just cli watch -p beet_site {{args}}

build-site *args:
	just cli build -p beet_site {{args}}


#💡 Test

min-stack := 'RUST_MIN_STACK=33554432'
test-threads:= '--test-threads=8'
# Run tests for ci, not using workspace cos somehow bevy_default still getting pulled in
# cargo test --workspace runs with 16MB stack and max 8 cores
# {{min-stack}} cargo test --workspace 			--lib											{{args}} -- {{test-threads}}
# {{min-stack}} cargo test --workspace 			--doc	--features=_doctest	{{args}} -- {{test-threads}}

test-fmt:
	cargo fmt 				--check
	just leptosfmt 		--check

test-ci *args:
	just test-fmt
	just test-web
	just test-flow

test-web *args:
	{{min-stack}} cargo test -p beet_design 	 	 																												{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_router 	--features=_test_site,build,serde,server,parser,bevy 			{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx 			--features=bevy,css,parser 																{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx_parser 																												{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx_combinator 																										{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_server 																														{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_site																																{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet-cli																																{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx 			--lib --features=bevy 		--target wasm32-unknown-unknown {{args}} -- {{test-threads}}
	
test-flow *args:
	{{min-stack}} cargo test -p beet_flow 		--features=_doctest,reflect 															{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_sim		 	--lib																											{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_spatial	--features=_doctest																				{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_flow 		--lib --features=reflect 	--target wasm32-unknown-unknown {{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_spatial 	--lib 									 	--target wasm32-unknown-unknown {{args}} -- {{test-threads}}

test-all-lib *args:
	{{min-stack}} cargo test --workspace 			--lib 	--all-features																	{{args}} -- {{test-threads}}
test-all-doc *args:
	{{min-stack}} cargo test --workspace 			--doc 	--all-features																	{{args}} -- {{test-threads}}

# rebuilding bevy_render for wasm results in 'no space left on device'
test-all *args:
	just test-ci
	just test-all-lib 																																								{{args}}
	just test-all-doc 																																								{{args}}
	{{min-stack}}	cargo test -p beet_rsx 			--lib 	--target wasm32-unknown-unknown --all-features  {{args}} -- {{test-threads}}
	{{min-stack}}	cargo test -p beet_flow 		--lib 	--target wasm32-unknown-unknown --all-features  {{args}} -- {{test-threads}}
	{{min-stack}}	cargo test -p beet_spatial 	--lib 	--target wasm32-unknown-unknown --all-features  {{args}} -- {{test-threads}}

#cargo test -p beet_spatial
#cargo test -p beet_sim
#cargo test -p beet_ml
# cargo test --workspace -- {{args}}
# cargo test --workspace --all-features -- {{args}}

test crate *args:
	just watch cargo test -p {{crate}} --lib -- 																								--watch {{args}}
test-doc crate *args:
	just watch cargo test -p {{crate}} --doc 																														{{args}}
test-e2e crate test_name *args:
	just watch cargo test -p {{crate}} --test {{test_name}} -- 																	--watch {{args}}
test-feat crate *args:
	just watch cargo test -p {{crate}} --lib --all-features -- 																					{{args}}
test-wasm crate *args:
	just watch cargo test -p {{crate}} --lib --target wasm32-unknown-unknown -- 								--watch {{args}}
test-wasm-feat crate *args:
	just watch cargo test -p {{crate}} --lib --target wasm32-unknown-unknown --all-features -- 					{{args}}
test-wasm-e2e crate test_name *args:
	just watch cargo test -p {{crate}} --test {{test_name}} --target wasm32-unknown-unknown -- 	--watch {{args}}
test-rsx-macro *args:
	just watch cargo test -p beet_rsx --test rsx_macro --features=css -- 												--watch {{args}}

# create codegen files
codegen:
	just clear-artifacts
	cargo run -p beet_router --example test_site_codegen
	just cli build -p beet_site

clear-artifacts:
	rm -rf target
	rm -rf crates/beet_design/src/codegen
	rm -rf crates/beet_router/src/test_site/codegen
	rm -rf crates/beet_site/src/codegen

# massive purge
purge:
	just clear-artifacts
	cargo clean
	rm -rf ./target
	rm -rf $CARGO_TARGET_DIR/rust-analyzer
# rm -rf ./Cargo.lock

pws *args:
	just --shell powershell.exe --shell-arg -c {{args}}

tree:
	cargo tree --depth=2 -e=no-dev

#💡 Misc

expand crate example *args:
	just watch 'cargo expand -p {{crate}} --example {{example}} {{args}}'

patch:
	cargo set-version --bump patch

publish crate *args:
	cargo publish -p {{crate}} --allow-dirty --no-verify {{args}}
	sleep 2

publish-all *args:
	just publish beet_flow_macros     {{args}} || true
	just publish beet_flow            {{args}} || true
	just publish beet_spatial         {{args}} || true
	just publish beet_ml              {{args}} || true
	just publish beet_server       		{{args}} || true
	just publish beet_sim          		{{args}} || true
	just publish beet_rsx_parser      {{args}} || true
	just publish beet_rsx_macros      {{args}} || true
	just publish beet_rsx             {{args}} || true
	just publish beet_router          {{args}} || true
	just publish beet_examples        {{args}} || true
	just publish beet                 {{args}} || true
	just publish beet-cli             {{args}} || true
# just publish beet_examples        {{args}} || true

watch *command:
	sweet watch \
	--include '**/*.rs' \
	--exclude '{.git,target,html}/**' \
	--exclude '*codegen*' \
	--cmd "{{command}}"

assets-push:
	aws s3 sync ./assets s3://bevyhub-public/assets --delete
	tar -czvf ./assets.tar.gz ./assets
	aws s3 cp ./assets.tar.gz s3://bevyhub-public/assets.tar.gz
	rm ./assets.tar.gz

assets-pull:
	curl -o ./assets.tar.gz https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
	tar -xzvf ./assets.tar.gz
	rm ./assets.tar.gz

#💡 Misc

# Cargo search but returns one line
search *args:
	cargo search {{args}} | head -n 1
