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

# Initialize the repository, pulling assets into their respective crates.
# Also we need to build the test_site codegen, which cant use a build script
# due to cyclic dependencies
init-repo:
	just init-flow
	just init-rsx

# mkdir -p ws_flow/beet_rsx/assets/fonts && cp ./assets/fonts/* ws_rsx/beet_rsx/assets/fonts
init-flow:
	just assets-pull
	mkdir -p ws_flow/beet_ml/assets/ml && cp ./assets/ml/default-bert.ron ws_flow/beet_ml/assets/ml/default.bert.ron

# once beet-cli is binstallable we shouldnt need to compile in order to codegen
init-rsx:
	cargo run -p beet_router --example build
	just cli build -p beet_site
	cd infra && npm ci
	mkdir -p target/lambda/crates/beet_site || true
	@echo 'dummy file so sst deploys' > target/lambda/crates/beet_site/bootstrap

init-sweet:
	just install-chromedriver

# just test-site
# just export-scenes

#💡 CLI

# Run a cli command as if it was installed
cli *args:
	cargo run -p beet-cli -- {{args}}

install-cli *args:
	cargo install --path crates/beet-cli {{args}}

sst-deploy:
	npx sst deploy --stage production --config infra/sst.config.ts

sst-remove:
	npx sst remove --stage production --config infra/sst.config.ts

deploy *args:
	just cli deploy 										\
	--package 				beet_site 				\
	--function-name 	BeetServerLambda	\
	{{args}}

# --region 					us-west-2 			\
# --iam-role 				$AWS_IAM_ROLE 	\

mod *args:
	sweet mod --exclude *codegen* {{args}}

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
	ws_rsx/beet_rsx/**/*.rs 					\
	ws_rsx/beet_rsx/**/**/*.rs 				\
	ws_rsx/beet_rsx/**/**/**/*.rs 		\
	ws_rsx/beet_design/**/*.rs 				\
	ws_rsx/beet_design/**/**/*.rs 		\
	ws_rsx/beet_design/**/**/**/*.rs 	\
	ws_rsx/beet_router/**/*.rs 				\
	ws_rsx/beet_router/**/**/*.rs 		\
	ws_rsx/beet_router/**/**/**/*.rs 	\
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

# it keeps asking for bigger stacks?
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

# upstream from sweet_test
test-fs *args:
	just watch 'cargo test -p sweet_fs --lib {{args}}'
# upstream from sweet_test
test-utils *args:
	just watch 'cargo test -p sweet_utils --lib --features=serde {{args}}'


# just test-flow runs out of space
test-build *args:
	{{min-stack}} cargo test -p beet_common 					--all-features																		{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx_combinator 	--all-features																		{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx_parser 			--all-features																		{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_build 						--all-features																		{{args}} -- {{test-threads}}

test-rsx *args:
	{{min-stack}} cargo test -p beet_build 	 	--features=bevy,style																			{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_design 	 	 																												{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_router 	--features=serde 																					{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p beet_rsx 			--features=bevy,css,parser 																{{args}} -- {{test-threads}}
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


test-sweet *args:
	{{min-stack}} cargo test -p sweet_bevy 							--features=rand 												 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet_fs 								--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet_net 							--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet_server 						--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet_test 			--lib 	--all-features  										 			{{args}} -- {{test-threads}} --e2e
	{{min-stack}} cargo test -p sweet_utils 						--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test -p sweet-cli 							--all-features 													 	{{args}} -- {{test-threads}}
	{{min-stack}} cargo test --lib --target wasm32-unknown-unknown --all-features -p sweet_bevy   {{args}} -- {{test-threads}}
	{{min-stack}} cargo test --lib --target wasm32-unknown-unknown --all-features -p sweet_test   {{args}} -- {{test-threads}}
	{{min-stack}} cargo test --lib --target wasm32-unknown-unknown --all-features -p sweet_web   	{{args}} -- {{test-threads}}

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
test-e2e crate *args:
	just watch cargo test -p {{crate}} --lib --features=e2e -- 														--e2e	--watch {{args}}
test-doc crate *args:
	just watch cargo test -p {{crate}} --doc 																														{{args}}
test-integration crate test_name *args:
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

clear-artifacts:
	rm -rf target
	rm -rf ws_rsx/beet_router/src/test_site/codegen
	rm -rf ws_rsx/beet_design/src/codegen
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
	just publish beet_rsx_combinator  {{args}} || true
	@echo 'Publishing Sweet Crates'
	just publish sweet_utils				{{args}} | true
	just publish sweet_fs						{{args}} | true
	just publish sweet_test_macros	{{args}} | true
	just publish sweet_test					{{args}} | true
	just publish sweet_server				{{args}} | true
	just publish sweet_web					{{args}} | true
	just publish sweet_bevy					{{args}} | true
	just publish sweet_net					{{args}} | true
	just publish sweet 							{{args}} | true
	just publish sweet-cli					{{args}} | true
	@echo 'Publishing Flow Crates'
	just publish beet_flow_macros     {{args}} || true
	just publish beet_flow            {{args}} || true
	just publish beet_spatial         {{args}} || true
	just publish beet_ml              {{args}} || true
	just publish beet_sim          		{{args}} || true
	just publish beet_examples        {{args}} || true
	@echo 'Publishing Rsx Build Crates'
	just publish beet_common      		{{args}} || true
	just publish beet_rsx_parser      {{args}} || true
	just publish beet_rsx_macros      {{args}} || true
	@echo 'Publishing Rsx Crates'
	just publish beet_rsx             {{args}} || true
	just publish beet_router          {{args}} || true
	just publish beet_server       		{{args}} || true
	just publish beet_connect      		{{args}} || true
	just publish beet_design 					{{args}} || true
	@echo 'Publishing Build Crates'
	@echo 'Publishing Misc Crates'
	just publish beet                 {{args}} || true
	just publish beet-cli             {{args}} || true



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

assets-pull:
	curl -o ./assets.tar.gz https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
	tar -xzvf ./assets.tar.gz
	rm ./assets.tar.gz

#💡 Misc

# Cargo search but returns one line
search *args:
	cargo search {{args}} | head -n 1

# Run a command with the sweet cli without installing it
sweet *args:
	cargo run -p sweet-cli -- {{args}}

# Install the sweet cli
install-sweet-cli *args:
	cargo install --path crates/sweet-cli {{args}}


# creates a directory ~/chrome-for-testing and installs chrome and chromedriver there.
# The latest version can be found at https://googlechromelabs.github.io/chrome-for-testing/
# Previous versions can be found at
install-chromedriver:
	wget https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.114/linux64/chrome-linux64.zip -P ~/chrome-for-testing
	wget https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.114/linux64/chromedriver-linux64.zip -P ~/chrome-for-testing
	mkdir -p ~/chrome-for-testing
	unzip ~/chrome-for-testing/chrome-linux64.zip -d ~/chrome-for-testing
	unzip ~/chrome-for-testing/chromedriver-linux64.zip -d ~/chrome-for-testing
	chmod +x ~/chrome-for-testing/chromedriver-linux64/chromedriver
	export PATH="$PATH:$HOME/chrome-for-testing/chromedriver-linux64"
	echo "Chrome installed at: $HOME/chrome-for-testing/chrome-linux64/chrome"
	echo "ChromeDriver installed at: $HOME/chrome-for-testing/chromedriver-linux64/chromedriver"
	echo "ChromeDriver version:"
	~/chrome-for-testing/chromedriver-linux64/chromedriver --version
