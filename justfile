set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_ecs'
testable := 'beet_ecs'

default:
	just --list





# mdbooks server is busted on wsl so I use live-server
book:
	cd docs && mdbook serve & cd docs/book && live-server --no-browser

env:
	@echo $RUST_LOG

publish crate *args:
	cargo publish -p {{crate}} --allow-dirty --no-verify {{args}}
	sleep 2

publish-all:
	just publish beet_ecs_macros
	just publish beet_ecs
	just publish beet

test crate *args:
	cargo run -p {{crate}} --example test_{{crate}} $BEET_CARGO -- {{args}}

test-w crate *args:
	just watch 'cargo run -p {{crate}} --example test_{{crate}} $BEET_CARGO -- -w {{args}}'

test-all *args:
	for crate in {{testable}}; do \
			just test $crate {{args}}; \
	done


test-ci crate *args:
	cargo run -p {{crate}} --example test_{{crate}} {{args}}

test-ci-all *args:
	for crate in {{testable}}; do \
			just test-ci $crate {{args}}; \
	done


watch *command:
	forky watch \
	-w '**/*.rs' \
	-i '{.git,target,html}/**' \
	-i '**/mod.rs' \
	-- {{command}}