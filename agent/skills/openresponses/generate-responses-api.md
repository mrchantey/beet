Implement the Open Responses API in `crates/beet_clanker/src/flow_agent/openresponses` using sensible Rust conventions:

- Separate code by files with all public items exposed via `mod.rs`
- Use beautiful Rust conventions, docs, and doc examples instead of relying on openapi auto-generation, which typically results in language-specific antipatterns
- Properly handle API requirements including `serde` renames where appropriate

## Testing

- Create the six integration tests in `crates/beet_clanker/tests/openresponses.rs`
An example integration test may look something like this (the exact API may differ):

```rust
use beet_clanker::prelude::*;
use beet_net::prelude::*;

#[beet_core::test]
async fn basic_text_response(){
Request::post("https://api.openai.com/v1/responses")
	.with_auth(env_ext::get("OPENAI_API_KEY"))
	.with_body(openresponses::request::Body{
		model: providers::openai::GPT_5_MINI,
		input:"respond exactly with the phrase 'foobar' and nothing else"})
	.send()
	.await()
	.into_result()
	.unwrap()
	.body::<openresponses::response::Body>().unwrap().items[0]
	.xpect_eq("foobar")
}
```

## Documentation

Add `no_run` doc examples to the api types as appropriate, demonstrating expected input and output. As doc test 
A doctest demonstrating part of the API would be similar to the integration tests, but use regular `assert!` instead of `xpect_eq`.
As these are no_run, use assertions on exact output values for illustration even though actual calls may not be so predictable.

## References

- [API reference](https://www.openresponses.org/reference)
- [Example compliance suite](skills/compliance-suite.md)
