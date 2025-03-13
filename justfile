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


doc crate *args:
	just watch cargo doc -p {{crate}} --open {{args}}

fmt *args:
	cargo fmt {{args}} && just leptosfmt {{args}}

leptosfmt *args:
	leptosfmt -q											\
	crates/beet_rsx/**/*.rs 					\
	crates/beet_rsx/**/**/*.rs 				\
	crates/beet_rsx/**/**/**/*.rs 		\
	crates/beet_router/**/*.rs 				\
	crates/beet_router/**/**/*.rs 		\
	crates/beet_router/**/**/**/*.rs 	\
	crates/beet_site/**/*.rs 					\
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


run-beet-site *args:
	just cli watch \
	-p beet_site \
	--mpa \
	--templates-root-dir crates/beet_site/src \
	--routes-dir crates/beet_site/src/routes 	\
	{{args}}

#💡 Test

min-stack := 'RUST_MIN_STACK=33554432'

# Run tests for ci,
# cargo test --workspace runs with 16MB stack and max 8 cores
test-ci *args:
	cargo fmt 				--check
	just leptosfmt 		--check
	{{min-stack}} cargo test --workspace --lib	--features=_doctest 			{{args}} -- --test-threads=8
	{{min-stack}} cargo test --workspace --doc	--features=_doctest 			{{args}} -- --test-threads=8
	cargo test --target wasm32-unknown-unknown 	--all-features	-p beet_flow 				{{args}} -- --test-threads=8

# rebuilding bevy_render for wasm results in 'no space left on device'
test-all *args:
	just test-ci 																																			{{args}}
	{{min-stack}} cargo test --workspace --lib 	--all-features							{{args}} -- --test-threads=8
	cargo test --lib --target wasm32-unknown-unknown --all-features -p beet_rsx 			{{args}}
	cargo test --lib --target wasm32-unknown-unknown --all-features -p beet_spatial 	{{args}}

#cargo test -p beet_spatial
#cargo test -p beet_sim
#cargo test -p beet_ml
# cargo test --workspace -- {{args}}
# cargo test --workspace --all-features -- {{args}}

test-doc crate *args:
	just watch 'cargo test -p {{crate}} --doc 						{{args}}'
# copied from sweet
test crate *args:
	just watch 'cargo test -p {{crate}} --lib -- --watch 	{{args}}'
test-e2e crate test_name *args:
	just watch 'cargo test -p {{crate}} --test {{test_name}} -- --watch {{args}}'
test-feat crate *args:
	just watch 'cargo test -p {{crate}} --lib --all-features -- {{args}}'
test-wasm crate *args:
	just watch 'cargo test -p {{crate}} --lib --target wasm32-unknown-unknown -- --watch {{args}}'
test-wasm-feat crate *args:
	just watch 'cargo test -p {{crate}} --lib --target wasm32-unknown-unknown --all-features -- {{args}}'
test-wasm-e2e crate test_name *args:
	just watch 'cargo test -p {{crate}} --test {{test_name}} --target wasm32-unknown-unknown -- --watch {{args}}'

serve-web:
	just serve-wasm

# massive purge
purge:
	cargo clean
	rm -rf ./target
	rm -rf $CARGO_TARGET_DIR/rust-analyzer
# rm -rf ./Cargo.lock

pws *args:
	just --shell powershell.exe --shell-arg -c {{args}}

tree:
	cargo tree --depth=2 -e=no-dev

#💡 WEB EXAMPLES

# Build a wasm example for the given crate and place the
# generated files in target/wasm-example/wasm
build-wasm crate example *args:
	cargo build -p {{crate}} --example {{example}} --target wasm32-unknown-unknown {{args}}
	wasm-bindgen \
	--out-dir ./target/wasm-example/wasm \
	--out-name bindgen \
	--target web \
	--no-typescript \
	~/.cargo_target/wasm32-unknown-unknown/debug/examples/{{example}}.wasm

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
	just publish beet_router_parser   {{args}} || true
	just publish beet_router          {{args}} || true
	just publish beet_examples        {{args}} || true
	just publish beet                 {{args}} || true
	just publish beet-cli             {{args}} || true
# just publish beet_examples        {{args}} || true

watch *command:
	sweet watch \
	--include '**/*.rs' \
	--exclude '{.git,target,html}/**' \
	--cmd "{{command}}"

copy-web-assets:
	mkdir -p target/wasm/assets || true
	cp -r ./assets/* target/wasm/assets


copy-wasm-assets:
	rm -rf ./target/static/assets
	mkdir -p ./target/static/assets || true
	
serve-wasm *args:
	cd ./target/static && forky serve {{args}}

watch-assets:
	just watch-web 'just copy-wasm-assets'

watch-web *command:
	forky watch \
	-w '**/*/assets/**/*' \
	-- {{command}}

assets-push:
	aws s3 sync ./assets s3://bevyhub-public/assets --delete
	tar -czvf ./assets.tar.gz ./assets
	aws s3 cp ./assets.tar.gz s3://bevyhub-public/assets.tar.gz
	rm ./assets.tar.gz

assets-pull:
	curl -o ./assets.tar.gz https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
	tar -xzvf ./assets.tar.gz
	rm ./assets.tar.gz

# https://gist.github.com/stephenhardy/5470814
# 1. Remove the history
# 2. recreate the repos from the current content only
# 3. push to the github remote repos ensuring you overwrite history
very-scary-purge-commit-history:
	rm -rf .git

	git init
	git add .
	git commit -m "Initial commit"

	git remote add origin git@github.com:mrchantey/beet.git
	git push -u --force origin main


#💡 Misc

# Cargo search but returns one line
search *args:
	cargo search {{args}} | head -n 1


#💡 Server

lambda-watch:
	cd crates/beet_server && cargo lambda watch --example lambda_axum


lambda-deploy *args:
	cargo lambda build 							\
	--package beet_site							\
	--features beet_server/lambda		\
	--release
	cargo lambda deploy			 		\
	beet 												\
	--binary-name beet_site			\
	--region us-west-2 					\
	--iam-role $AWS_IAM_ROLE 		\
	--enable-function-url 			\
	--include target/client 		\
	{{args}}