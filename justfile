set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_ecs beet_net'

default:
	just --list


## common
test-core *args:
	just watch 'cargo run -p beet_core 	--example test_beet_core --features="beet_net/tokio" 		$BEET_CARGO_TEST -- -w {{args}}'

test-ecs *args:
	just watch 'cargo run -p beet_ecs 	--example test_beet_ecs 														$BEET_CARGO_TEST -- -w {{args}}'

test-net *args:
	just watch 'cargo run -p beet_net 	--example test_beet_net --features="beet_net/tokio" $BEET_CARGO_TEST -- -w {{args}}'




# mdbooks server is busted on wsl so I use live-server
book:
	cd docs && mdbook serve & cd docs/book && live-server --no-browser

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
	cargo run -p {{crate}} --example test_{{crate}} $BEET_CARGO_TEST -- {{args}}

test-w crate *args:
	just watch 'cargo run -p {{crate}} --example test_{{crate}} $BEET_CARGO_TEST -- -w {{args}}'

test-all *args:
	cargo run -p beet_core 	--example test_beet_core --features="beet_net/tokio" $BEET_CARGO_TEST -- {{args}}
	cargo run -p beet_ecs 	--example test_beet_ecs 														 $BEET_CARGO_TEST -- {{args}}
	cargo run -p beet_net 	--example test_beet_net  --features="beet_net/tokio" $BEET_CARGO_TEST -- {{args}}

test-ci *args:
	cargo run -p beet_core --example test_beet_core --features="beet_net/tokio" $BEET_CARGO_TEST_CI -- {{args}}
	cargo run -p beet_ecs  --example test_beet_ecs 															$BEET_CARGO_TEST_CI -- {{args}}
	cargo run -p beet_net  --example test_beet_net  --features="beet_net/tokio" $BEET_CARGO_TEST_CI -- {{args}}

test-wasm crate *args:
	sweet -p {{crate}} --example test_{{crate}} --interactive --watch {{args}}

watch *command:
	forky watch \
	-w '**/*.rs' \
	-i '{.git,target,html}/**' \
	-i '**/mod.rs' \
	-- {{command}}