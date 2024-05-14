set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_core beet_ecs beet_net'

default:
	just --list --unsorted


## common

doc:
	just watch 'cargo doc'

serve-doc:
	cd ./target/doc/beet && forky serve


test-core *args:
	just watch 'cargo test -p beet_core --lib -- {{args}}'

test-ecs *args:
	just watch 'cargo test -p beet_ecs --lib -- {{args}}'
# just watch 'cargo test -p beet_ecs --lib -- --nocapture {{args}}'

test-ml *args:
	just watch 'cargo test -p beet_ml --lib -- {{args}}'


test-web *args:
	just test-wasm beet_web {{args}}

build-web *args:
	just copy-wasm-assets
	just build-wasm-release beet_web main

run-web *args:
	just copy-wasm-assets
	just watch-wasm-debug beet_web main

run crate example *args:
	cargo run -p {{crate}} --example {{example}} {{args}}

run-w crate example *args:
	just watch 'just run {{crate}} {{example}} {{args}}'

watch-ml-example:
	just watch 'just build-web-example beet hello_ml'

build-web-examples:
	rm -rf ./target/web-examples || true
	just build-web-example animation
	just build-web-example flock
	just build-web-example hello_world
	just build-web-example hello_ml
	just build-web-example seek

serve-web-examples:
	cd ./target/web-examples && forky serve

deploy-web-examples:
	just build-web-examples
	gsutil -m rsync -d -r ./target/web-examples gs://beet-examples
# -m parallel rsync copy -d delete if not in local -r recursive

build-web-example example *args:
	mkdir -p ./target/web-examples/{{example}} || true
	mkdir -p ./target/web-examples/{{example}}/assets || true
	cp -r ./examples/html/* ./target/web-examples/{{example}}
	cp -r ./assets/* ./target/web-examples/{{example}}/assets
	cargo build --example {{example}} --target wasm32-unknown-unknown --release {{args}}
	wasm-bindgen \
	--out-name main \
	--out-dir ./target/web-examples/{{example}}/wasm \
	--target web \
	./target/wasm32-unknown-unknown/release/examples/{{example}}.wasm \
	--no-typescript \

serve-web:
	just serve-wasm

book:
	cd docs && mdbook serve

# mdbooks server is busted on wsl so I use live-server
serve-book:
	cd docs/book && live-server --no-browser

clean-repo:
	cargo clean
	rm -rf ./target
# rm -rf ./Cargo.lock

env:
	@echo $RUST_LOG

expand crate example *args:
	just watch 'cargo expand -p {{crate}} --example {{example}} {{args}}'

publish crate *args:
	cargo publish -p {{crate}} --allow-dirty --no-verify {{args}}
	sleep 2

publish-all:
	just publish beet_ecs_macros
	just publish beet_ecs
	just publish beet

test crate *args:
	cargo run -p {{crate}} --example test_{{crate}} -- {{args}}

test-w crate *args:
	just watch 'cargo run -p {{crate}} --example test_{{crate}} -- -w {{args}}'

test-all:
	cargo test -p beet_ecs --lib
	cargo test -p beet_core --lib
	cargo test -p beet_net --lib

test-ci *args:
	cargo run -p beet_core --example test_beet_core -- {{args}}
	cargo run -p beet_ecs  --example test_beet_ecs  -- {{args}}
	cargo run -p beet_net  --example test_beet_net  -- {{args}}

test-wasm crate *args:
	sweet -p {{crate}} --example test_{{crate}} --interactive --watch {{args}}

watch *command:
	forky watch \
	-w '**/*.rs' \
	-i '{.git,target,html}/**' \
	-i '**/mod.rs' \
	-- {{command}}




build-wasm-release crate example *args:
	just _build-wasm release {{crate}} {{example}} --release {{args}}
build-wasm-debug crate example *args:
	just _build-wasm debug {{crate}} {{example}} {{args}}
watch-wasm-release crate example *args:
	just _watch-wasm release {{crate}} {{example}} --release {{args}}
watch-wasm-debug crate example *args:
	just _watch-wasm debug {{crate}} {{example}} {{args}}

_build-wasm build_config crate example *args:
	cargo build -p {{crate}} --example {{example}} --target wasm32-unknown-unknown {{args}}
	wasm-bindgen \
	--out-dir ./target/static/wasm \
	--target web \
	./target/wasm32-unknown-unknown/{{build_config}}/examples/{{example}}.wasm \
	--no-typescript \

_watch-wasm build_config crate example *args:
	just watch 'just _build-wasm {{build_config}} {{crate}} {{example}} {{args}}'

copy-wasm-assets:
	rm -rf ./target/static/assets
	mkdir -p ./target/static/assets || true
	cp -r ./crates/beet_web/assets/* ./target/static

	
serve-wasm *args:
	cd ./target/static && forky serve {{args}}

# npx live-server \

# --no-browser \

# --host=0.0.0.0 \

# --watch=wasm/site_bg.wasm,wasm/simulator_bg.wasm,index.html,style.css \

watch-assets:
	just watch-web 'just copy-wasm-assets'

watch-web *command:
	forky watch \
	-w '**/*/assets/**/*' \
	-- {{command}}



# too scary with untracked changes etc, do it manually
# deploy-web:
#		just build-web
# 	rm -rf /tmp/beet
# 	mkdir -p /tmp/beet || true
# 	cp -r target/static/* /tmp/beet
# 	git checkout pages
# 	mkdir -p play || true
# 	cp -r /tmp/beet/* play
# 	git add .
# 	git commit -m "Publish Playground"
# 	git push origin main
# 	git checkout main