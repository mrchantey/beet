set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_core beet_ecs beet_net'

default:
	just --list --unsorted


## common
cmd *args:
	cd /cygdrive/c/work/beet && {{args}}

build-scenes *args:
	cargo run -p beet_examples --example build_scenes {{args}}
app *args:
	cargo run --example app_full {{args}}

doc:
	just watch 'cargo doc'

serve-doc:
	cd ./target/doc/beet && forky serve

test-all *args:
	just watch 'cargo test --workspace --lib -- {{args}}'

test-core *args:
	just watch 'cargo test -p beet_core --lib -- {{args}}'

test-ecs *args:
	just watch 'cargo test -p beet_ecs --lib -- {{args}}'
# just watch 'cargo test -p beet_ecs --lib -- --nocapture {{args}}'

test-ml *args:
	just watch 'cargo test -p beet_ml --lib -- {{args}}'

test-net *args:
	just watch 'cargo test -p beet_net --lib -- {{args}}'

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

serve-web:
	just serve-wasm

book:
	cd docs && mdbook serve --port 3001

# mdbooks server is busted on wsl so I use live-server
serve-book:
	cd docs/book && live-server --no-browser

clean-repo:
	cargo clean
	rm -rf ./target
# rm -rf ./Cargo.lock

powershell *args:
	just --shell powershell.exe --shell-arg -c {{args}}

#### WEB EXAMPLES #####################################################

wasm-dir:= './target/web-examples'
web-examples:= 'animation app_basics app_full fetch flock frozen_lake_run frozen_lake_train hello_ml hello_world seek_3d seek'

web-example-build example *args:
	mkdir -p {{wasm-dir}}/{{example}} || true
	cargo build --example {{example}} --target wasm32-unknown-unknown --release {{args}}
	wasm-bindgen \
	--out-name main \
	--out-dir {{wasm-dir}}/{{example}} \
	--target web \
	$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/examples/{{example}}.wasm \
	--no-typescript \

web-example-watch example *args:
	just copy-web-assets
	just watch 'just web-example-build {{example}} {{args}}'

web-examples-build-and-deploy:
	just web-examples-build
	just web-examples-deploy

copy-web-assets:
	mkdir -p {{wasm-dir}}/assets || true
	cp -r ./assets/* {{wasm-dir}}/assets

web-examples-build:
	rm -rf {{wasm-dir}} || true
	just copy-web-assets
	for file in {{web-examples}}; do \
		just web-example-build ${file}; \
	done

web-examples-size:
	for file in {{web-examples}}; do \
		du -sh {{wasm-dir}}/${file}; \
	done

web-examples-serve:
	cd {{wasm-dir}} && forky serve --any-origin --port=3002

web-examples-deploy:
	gsutil -m rsync -c -d -r {{wasm-dir}} gs://beet-examples
# -m  		= parallel 
# -c 			= use checksum instead of timestamp for compare
# rsync  	=	copy 
# -d 			= delete if not in local 
# -r 			= recursive


### MISC

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
build-wasm crate example *args:
	just _build-wasm debug {{crate}} {{example}} {{args}}
watch-wasm-release crate example *args:
	just _watch-wasm release {{crate}} {{example}} --release {{args}}
watch-wasm crate example *args:
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

### GSUTIL
push-assets:
	tar -czvf ./assets.tar.gz ./assets
	gsutil cp ./assets.tar.gz gs://beet-misc/assets.tar.gz
	gsutil cp ./assets.tar.gz gs://beet-misc/assets-backup.tar.gz
	rm ./assets.tar.gz

pull-assets:
	curl -o ./assets.tar.gz https://storage.googleapis.com/beet-misc/assets.tar.gz
	tar -xzvf ./assets.tar.gz
	rm ./assets.tar.gz

view-cors:
	gcloud storage buckets describe gs://beet-examples --format="default(cors_config)"
update-cors:
	gcloud storage buckets update gs://beet-examples --cors-file=cors.json