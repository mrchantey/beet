# beet_net refactor

Time to give beet_net a bit of a refactor:

- RequestParts::version - Cow<'static,str> its very rarely used as a custom value

## `url.rs`

Currently the url is hardcoded into RequestParts but we should break this out into its own type and module.

The url crate is for super low latency zero copy etc but we're an application framework prioritizing ease of use.

When parsing we should allow the two slashes to be omitted, ie `http:example.com` is fine, but include them when rendering url to string.

You may need to do some of your own design work here, treat this as a guide.

A lot of the documentation references http vs cli, thats unnessecary just make the language a bit agnostic thats all.

```rust
pub struct Url{
	scheme: Scheme,
	authority: Option<String>,
	path: Vec<String>,
	params: MultiMap<String,String>,
	fragment: Option<String>,
}

// should we include 'non-schemes' like absolute path, relative path, network path?
// i dont think none should be allowed, i think none means absolute?
// remove the Cli and repl schemes, these are categorically incorrect, that stuff should be in headers
// we also need file scheme.
enum Scheme{
	
}
```


## `media_type.rs`

MimeType found in header_map.rs is an outdated term, replace with MediaType and put in a seperate module. update usage and docs to reflect this, including changing `mime` variables to be `media_type`.


Verify all changes succeeded first by testing the beet_net module then with `just test-core`. note we only care about the following pacakges, do not update or change any others.

beet_core
beet_net
beet_node
beet_tool
beet_stack
