## Task

We will now do a comprehensive code quality pass for the core crates of this project. This involves combing through a crate file by file to ensure its quality.

This task has the primary goal of improving public apis:
1. Reduce usage of `pub`: prefer `pub(super)`, `pub(crate)` etc wherever it makes sense, this includes at the module level and for struct fields.
2. Public Docs: ensure all public apis are well documented according to RFC 1574, except where it would be overly verbose.

While we are passing through source files we should also consider two secondary goals:
3. Comments: ensure all complex logic explains well its behavior
4. Tests: ensure all tests are of high quality
5. Integration tests and examples: these should also be well documented. examples should be in root directory, not individual crates.

**QUALITY NOT QUANTITY**
LLMs love writing text, if its overly verbose or explaining the obvious this has negative value. Ensure only whats needed is used, ie `MyStruct::new` *usually* does not need an example, just a one-liner doc. General usage should be in the top level `MyStruct` docs
Many modules will need module-level documentation. try not to duplicate struct and module level docs. the module level should contain very high level concepts and most modules dont need examples.

## Verification

When finished a pass verifying a crate:
1. add `#[deny(missing_docs)`, fix errors until cargo check passes. not negotiable!
2. Run tests with all features, both native with docs, and wasm lib only, ie:
	- `cargo test -p beet_core --all-features`
	- `cargo test -p beet_core --all-features --lib --target=wasm32-unknown-unkown`
	- use timeout and tail to avoid hangs and preserve context


## Crates

We will start from the basic dependencies and work upwards. Skip those marked completed.

- [X] beet_core
- [X] beet_flow
- [X] beet_net
- [ ] beet_router
- [ ] beet_parse
- [ ] beet_build
- [ ] beet_dom


Do not ask to continue to the next one, just do it.

We can verify all changes by running: `just test-all`
