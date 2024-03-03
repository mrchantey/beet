set windows-shell := ["C:/tools/cygwin/bin/sh.exe","-c"]
set dotenv-load
crates := 'beet beet_ecs'

default:
	just --list

env:
	@echo $RUST_LOG

test crate *args:
	cargo run -p {{crate}} --example test_{{crate}} $BEET_CARGO -- {{args}}

publish crate *args:
	cargo publish -p {{crate}} --allow-dirty --no-verify {{args}}
	sleep 2


publish-all:
	just publish beet_ecs_macros
	just publish beet_ecs
	just publish beet

# test-w crate *args:
# 	{{env}} just watch 'cargo run -p {{crate}} --example test_{{crate}} {{target}} {{features}} -- -w {{args}}'
