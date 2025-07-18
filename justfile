# Command runners are great for individual projects
# but as a metaframework these are all signs of painpoints in the workflow
# arguments should be config files, shell command chains should be beet subcommands etc
# 
# Anyway for now this is useful, but if you feel like making a change here, consider instead
# adding a subcommand to the beet or sweet cli, and describing it in the cli readme.
# 
# ```rust 
# cargo binstall just
# just --list
# just test-ci
# ```
#

set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_spatial beet_flow'
# max cargo build jobs

default:
	just --list --unsorted

#💡 Init

# Initialize the repository, pulling assets into their respective crates.
# Also we need to build the codegen files for rsx crates like beet_design.
init-repo:
	just init-flow
	just init-rsx

init-flow:
	just assets-pull
	mkdir -p crates/beet_ml/assets/ml && cp ./assets/ml/default-bert.ron crates/beet_ml/assets/ml/default.bert.ron

# once beet-cli is binstallable we shouldnt need to compile in order to codegen
init-rsx:
	just cli build -p beet_site
	cd infra && npm ci
	mkdir -p target/lambda/crates/beet_site || true
	@echo 'dummy file so sst deploys' > target/lambda/crates/beet_site/bootstrap

init-sweet:
	just install-chromedriver

assets-pull:
	curl -o ./assets.tar.gz https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
	tar -xzvf ./assets.tar.gz
	rm ./assets.tar.gz

# just test-site
# just export-scenes

#💡 CLI

# Run a cli command as if it was installed
cli *args:
	cargo run -p beet-cli -- {{args}}

install-cli *args:
	cargo install --path crates/beet-cli {{args}}

lambda-build:
	cargo lambda build -p beet_site --features beet/lambda --release --lambda-dir target/lambda/crates



mod *args:
	just sweet mod --exclude */codegen/* {{args}}

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

run-csr:
	just watch just build-csr

build-csr:
	cargo run --example csr
	cargo build --example csr --target-dir=target --features=rsx --target wasm32-unknown-unknown
	wasm-bindgen --out-dir target/examples/csr/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/csr.wasm
	sweet serve target/examples/csr
	
run-hydration:
	just watch just build-hydration

build-hydration:
	cargo run --example hydration
	cargo build --example hydration --target-dir=target --features=rsx --target wasm32-unknown-unknown
	wasm-bindgen --out-dir target/examples/hydration/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/hydration.wasm
	sweet serve target/examples/hydration

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

#just cli watch -p beet_site {{args}}
build-site *args:
	just cli build -p beet_site {{args}}

run-site *args:
	just cli run -p beet_site {{args}}

deploy-site *args:
	just cli deploy -p beet_site --release


#💡 Test

# it keeps asking for bigger stacks? a mold thing?
min-stack := 'RUST_MIN_STACK=134217728'
# min-stack := 'RUST_MIN_STACK=67108864'
# min-stack := 'RUST_MIN_STACK=33554432'
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
	just test-rsx

# upstream from sweet
test-fs *args:
	just watch 'cargo test -p beet_utils --lib --features fs -- --nocapture {{args}}'
# upstream from sweet
test-beet-utils *args:
	just watch 'cargo test -p beet_utils --lib --features=serde --nocapture -- {{args}}'

test-rsx *args:
	{{min-stack}} cargo test -p beet_rsx_combinator 	--all-features																			{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_parse 						--all-features 	 	 																	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx_macros 	--all-features 	 	 																	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx					 	 	 																								{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_router						--all-features 	 	 																	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_server						--all-features 	 	 																	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_build 						--all-features																			{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet-cli 							--all-features																			{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_design 					--all-features																			{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_site 						--all-features																			{{args}} -- {{test-threads}}
	
test-flow *args:
	{{min-stack}} cargo test -p beet_flow 		--features=_doctest,reflect 															{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_sim		 	--lib																											{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_spatial	--features=_doctest																				{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_flow 		--lib --features=reflect 	--target wasm32-unknown-unknown {{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_spatial 	--lib 									 	--target wasm32-unknown-unknown {{args}} -- {{test-threads}}


#{{min-stack}} cargo test -p sweet 			--lib 	--all-features  										 			{{args}} -- {{test-threads}} --e2e
test-utils *args:
	{{min-stack}} cargo test -p beet_utils 							--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet 									 													 								{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet-cli 							--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet     --lib --target wasm32-unknown-unknown  --all-features   {{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_core_macros 				--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_core 							--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_core --lib --target wasm32-unknown-unknown  --all-features   {{args}} -- {{test-threads}}

test-all-lib *args:
	{{min-stack}} cargo test --workspace 			--lib 	--all-features																	{{args}} -- {{test-threads}}
test-all-doc *args:
	{{min-stack}} cargo test --workspace 			--doc 	--all-features																	{{args}} -- {{test-threads}}

test-all:
	just test-utils
	just test-flow
	just test-rsx

# rebuilding bevy_render for wasm results in 'no space left on device'
test-all-old *args:
	just test-ci
	just test-all-lib 																																								{{args}}
	just test-all-doc 																																								{{args}}
	{{min-stack}}	cargo test -p beet_rsx	--lib 	--target wasm32-unknown-unknown --all-features  {{args}} -- {{test-threads}}
	{{min-stack}}	cargo test -p beet_flow 		--lib 	--target wasm32-unknown-unknown --all-features  {{args}} -- {{test-threads}}
	{{min-stack}}	cargo test -p beet_spatial 	--lib 	--target wasm32-unknown-unknown --all-features  {{args}} -- {{test-threads}}

#cargo test -p beet_spatial
#cargo test -p beet_sim
#cargo test -p beet_ml
# cargo test --workspace -- {{args}}
# cargo test --workspace --all-features -- {{args}}

test crate *args:
	sweet test -p {{crate}} --lib --watch {{args}}
test-int crate test *args:
	sweet test -p {{crate}} --watch --test {{test}} {{args}}
test-e2e crate *args:
	just watch cargo test -p {{crate}} --lib --features=e2e -- 														--e2e	--watch {{args}}
test-doc crate *args:
	just watch cargo test -p {{crate}} --doc 																														{{args}}
test-wasm crate *args:
	just watch cargo test -p {{crate}} --lib --target wasm32-unknown-unknown -- 								--watch {{args}}
test-wasm-feat crate *args:
	just watch cargo test -p {{crate}} --lib --target wasm32-unknown-unknown --all-features -- 					{{args}}
test-wasm-e2e crate test_name *args:
	just watch cargo test -p {{crate}} --test {{test_name}} --target wasm32-unknown-unknown -- 	--watch {{args}}
test-rsx-macro *args:
	just watch cargo test -p beet_rsx --test rsx_macro --features=css -- 												--watch {{args}}

clear-rust-analyzer:
	rm -rf $CARGO_TARGET_DIR/rust-analyzer

clear-ice:
	rm -f rustc-ice-*

clear-artifacts:
	just clear-ice
	rm -rf target
	rm -rf crates/beet_router/src/test_site/codegen
	rm -rf crates/beet_design/src/codegen
	rm -rf crates/beet_site/src/codegen

# massive purge
clear-all:
	just clear-artifacts
	cargo clean
	rm -rf $CARGO_TARGET_DIR
# rm -rf ./Cargo.lock

pws *args:
	just --shell powershell.exe --shell-arg -c {{args}}

tree:
	cargo tree --depth=2 -e=no-dev

#💡 Misc

expand crate test *args:
	just watch 'cargo expand -p {{crate}} --test {{test}} {{args}}'

patch:
	cargo set-version --bump patch

publish crate *args:
	cargo publish -p {{crate}} --allow-dirty --no-verify {{args}}

publish-all *args:
	@echo 'Publishing Utility Crates'
	just publish beet_utils						{{args}} | true
	just publish sweet_macros					{{args}} | true
	just publish sweet								{{args}} | true
	just publish beet_core_macros			{{args}} | true
	just publish beet_core						{{args}} | true
	just publish beet_connect					{{args}} | true
	just publish sweet-cli						{{args}} | true
	@echo 'Publishing Rsx Crates'
	just publish beet_rsx_combinator  {{args}} || true
	just publish beet_parse      			{{args}} || true
	just publish beet_rsx_macros      {{args}} || true
	just publish beet_rsx        			{{args}} || true
	just publish beet_router          {{args}} || true
	just publish beet_build      			{{args}} || true
	just publish beet_server       		{{args}} || true
	just publish beet_design 					{{args}} || true
	@echo 'Publishing Flow Crates'
	just publish beet_flow_macros     {{args}} || true
	just publish beet_flow            {{args}} || true
	just publish beet_spatial         {{args}} || true
	just publish beet_ml              {{args}} || true
	just publish beet_sim          		{{args}} || true
	just publish beet_examples        {{args}} || true
	@echo 'Publishing Top Crates'
	just publish beet                 {{args}} || true
	just publish beet-cli             {{args}} || true

#just publish beet_connect      		{{args}} || true
# just publish beet_examples        {{args}} || true

watch *command:
	sweet watch \
	--include '**/*.rs' \
	--exclude '{.git,target,html}/**' \
	--exclude '*/codegen/*' \
	--cmd "{{command}}"

assets-push:
	aws s3 sync ./assets s3://bevyhub-public/assets --delete
	tar -czvf ./assets.tar.gz ./assets
	aws s3 cp ./assets.tar.gz s3://bevyhub-public/assets.tar.gz
	rm ./assets.tar.gz

#💡 Misc

# Cargo search but returns one line
search *args:
	cargo search {{args}} | head -n 1

# Run a command with the sweet cli without installing it
sweet *args:
	cargo run -p sweet-cli -- {{args}}

# Install the sweet cli
install-sweet *args:
	cargo install --path crates/sweet/cli {{args}}


# creates a directory ~/chrome-for-testing and installs chrome and chromedriver there.
# The latest version can be found at https://googlechromelabs.github.io/chrome-for-testing/
# Previous versions can be found at
install-chromedriver:
	wget https://storage.googleapis.com/chrome-for-testing-public/137.0.7151.68/linux64/chrome-linux64.zip -P ~/chrome-for-testing
	wget https://storage.googleapis.com/chrome-for-testing-public/137.0.7151.68/linux64/chromedriver-linux64.zip -P ~/chrome-for-testing
	mkdir -p ~/chrome-for-testing
	unzip ~/chrome-for-testing/chrome-linux64.zip -d ~/chrome-for-testing
	unzip ~/chrome-for-testing/chromedriver-linux64.zip -d ~/chrome-for-testing
	chmod +x ~/chrome-for-testing/chromedriver-linux64/chromedriver
	export PATH="$PATH:$HOME/chrome-for-testing/chromedriver-linux64"
	echo "Chrome installed at: $HOME/chrome-for-testing/chrome-linux64/chrome"
	echo "ChromeDriver installed at: $HOME/chrome-for-testing/chromedriver-linux64/chromedriver"
	echo "ChromeDriver version:"
	~/chrome-for-testing/chromedriver-linux64/chromedriver --version
