set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_spatial beet_flow'

default:
	just --list --unsorted

run example *args:
	cargo run --example {{example}} {{args}}

run-w example *args:
	just watch 'just run {{example}} {{args}}'

run-p crate example *args:
	cargo run -p {{crate}} --example {{example}} {{args}}


## common
cmd *args:
	cd /cygdrive/c/work/beet && {{args}}

export-scenes *args:
	cargo run --example export_scenes {{args}}

app *args:
	cargo run --example app {{args}}

# blocked on #https://github.com/bevyengine/bevy/issues/14300
hello-world:
	just app \
	./scenes/beet-debug.json \
	../beetmash/scenes/camera-2d.json \
	../beetmash/scenes/ui-terminal-input.json \
	./scenes/hello-world.json

doc:
	just watch 'cargo doc'

serve-doc:
	cd ./target/doc/beet && forky serve

test-all *args:
	just watch 'cargo test --workspace --lib -- {{args}}'

test-spatial *args:
	just watch 'cargo test -p beet_spatial --lib -- {{args}}'

test-ecs *args:
	just watch 'cargo test -p beet_flow --lib -- {{args}}'
# just watch 'cargo test -p beet_flow --lib -- --nocapture {{args}}'

test-ml *args:
	just watch 'cargo test -p beet_ml --lib -- {{args}}'

test-examples *args:
	just watch 'cargo test -p beet_examples --lib -- {{args}}'

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

pws *args:
	just --shell powershell.exe --shell-arg -c {{args}}

#### WEB EXAMPLES #####################################################


# Build wasm files, pass --no-build to just update scenes and registries
build-wasm *args:
	just export-scenes
	beetmash build \
	--example app \
	--release \
	--copy-local ../beetmash-apps \
	--copy-scenes scenes \
	--copy-registries target/registries {{args}}
	beetmash build \
	--example app_ml \
	--release \
	--copy-local ../beetmash-apps {{args}}

### MISC

expand crate example *args:
	just watch 'cargo expand -p {{crate}} --example {{example}} {{args}}'

patch:
	cargo set-version --bump patch

publish crate *args:
	cargo publish -p {{crate}} --allow-dirty --no-verify {{args}}
	sleep 2

publish-all *args:
	just publish beet_flow_macros {{args}}	|| true
	just publish beet_flow {{args}}					|| true
	just publish beet_spatial {{args}}			|| true
	just publish beet_ml {{args}}						|| true
	just publish beet_examples {{args}}			|| true
	just publish beet {{args}}							|| true

test-wasm crate *args:
	sweet -p {{crate}} --example test_{{crate}} --interactive --watch {{args}}

watch *command:
	forky watch \
	-w '**/*.rs' \
	-i '{.git,target,html}/**' \
	-i '**/mod.rs' \
	-- {{command}}

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
	tar -czvf ./assets.tar.gz ./assets
	aws s3 cp ./assets.tar.gz s3://beetmash-public/assets.tar.gz
	rm ./assets.tar.gz

assets-pull:
	curl -o ./assets.tar.gz https://beetmash-public.s3.us-west-2.amazonaws.com/assets.tar.gz
	tar -xzvf ./assets.tar.gz
	rm ./assets.tar.gz

view-cors:
	gcloud storage buckets describe gs://beet-examples --format="default(cors_config)"
update-cors:
	gcloud storage buckets update gs://beet-examples --cors-file=cors.json















### TEST SCENE LOADS

test-fetch:
	cargo run --example app_ml \
	../beetmash/scenes/ui-terminal-input.json \
	../beetmash/scenes/lighting-3d.json \
	../beetmash/scenes/ground-3d.json \
	./scenes/beet-debug.json \
	./scenes/fetch-scene.json \
	./scenes/fetch-npc.json \


test-flock:
	cargo run --example app \
	../beetmash/scenes/camera-2d.json \
	../beetmash/scenes/space-scene.json \
	./scenes/beet-debug.json \
	./scenes/flock.json \

test-seek:
	cargo run --example app \
	../beetmash/scenes/camera-2d.json \
	../beetmash/scenes/space-scene.json \
	./scenes/beet-debug.json \
	./scenes/seek.json \

test-frozen-lake-train:
	cargo run --example app_ml \
	../beetmash/scenes/lighting-3d.json \
	./scenes/frozen-lake-scene.json \
	./scenes/frozen-lake-train.json \

test-frozen-lake-run:
	cargo run --example app_ml \
	../beetmash/scenes/lighting-3d.json \
	./scenes/frozen-lake-scene.json \
	./scenes/frozen-lake-run.json \

test-hello-animation:
	cargo run --example app \
	../beetmash/scenes/ui-terminal.json \
	../beetmash/scenes/lighting-3d.json \
	../beetmash/scenes/ground-3d.json \
	./scenes/beet-debug.json \
	./scenes/hello-animation.json \

test-hello-ml:
	cargo run --example app_ml \
	../beetmash/scenes/camera-2d.json \
	../beetmash/scenes/ui-terminal-input.json \
	./scenes/beet-debug.json \
	./scenes/hello-ml.json \

test-hello-world:
	cargo run --example app \
	../beetmash/scenes/camera-2d.json \
	../beetmash/scenes/ui-terminal.json \
	./scenes/beet-debug.json \
	./scenes/hello-world.json \

test-seek-3d:
	cargo run --example app \
	../beetmash/scenes/ui-terminal.json \
	../beetmash/scenes/lighting-3d.json \
	../beetmash/scenes/ground-3d.json \
	./scenes/beet-debug.json \
	./scenes/seek-3d.json \
