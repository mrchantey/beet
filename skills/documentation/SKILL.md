## Task

We will now do a comprehensive code quality pass for the core crates of this project.

This task has the primary goal of improving public apis:
1. Reduce usage of `pub`: prefer `pub(super)`, `pub(crate)` etc wherever it makes sense, this includes at the module level and for struct fields.
2. Public Docs: ensure all public apis are well documented according to RFC 1574, except where it would be overly verbose.

While we are passing through source files we should also consider two secondary goals:
3. Comments: ensure all complex logic explains well its behavior
4. Tests: ensure all tests are of high quality
5. Integration tests and examples: these should also be well documented. examples should be in root directory, not individual crates.

## Conventions

- **QUALITY NOT QUANTITY**: LLMs love writing text, but if its overly verbose or explaining the obvious, this has a negative value. Ensure only whats needed is used, ie `MyStruct::new` *usually* does not need an example, just a one-liner doc. General usage should be in the top level `MyStruct` docs
- Many modules will need module-level documentation. try not to duplicate struct docs in the module level. The module level should contain very high level concepts and most modules dont need examples.
- Do not discard comments, todos unless certain they are no longer relevent
- 
## Process

For each crate do the following:

1. add `#![deny(missing_docs)`
2. add docs until `cargo check` passes with all features, both native and wasm:
	- `cargo check -p beet_core --all-features`
	- `cargo check -p beet_core --all-features --target wasm32-unknown-unknown`
3. Run doc tests with all features:
- `timeout 1m cargo test -p beet_core --doc --all-features | tail 30`
4. mark the crate as done in this file: `skills/rust/beet-doc.md`

## Crates

We will start from the basic dependencies and work upwards. Skip those marked completed. Do not ask to continue to the next one, just do it.

- [ ] beet_core
- [ ] beet_flow
- [ ] beet_net
- [ ] beet_router
- [ ] beet_parse
- [x] beet_build
- [x] beet_dom


Finally we can verify all changes by running: `just test-all`
