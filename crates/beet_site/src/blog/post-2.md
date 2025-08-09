+++
title = "The Full Moon Harvest #2"
+++

# The Full Moon Harvest #2

The stack bevyfication continues this harvest, with ecs proving to be the anything paradigm!

## A Very Bevy Router

Servers like axum prioritize performance in the order of microseconds, with use-cases like proxy servers handling 10,000 requests per second. Web frameworks have different requirements, preferring cdns and s3 redirects over hitting the server, and averaging 200ms roundtrips when they do. 

`beet_server` addresses the ergonomics of webframework routing with features like multiple rendering strategies (ssr,ssg) and middleware beyond opaque `Request/Response` types.

```rust

fn routes() -> impl Bundle {
  children![
    (
      PathFilter::new("/hello"),
      bundle_endpoint(HttpMethod::Get, || rsx!{<div>hello world!</div>}),
    ),
    (
      HandlerConditions::fallback(),
      bundle_endpoint(HttpMethod::Get, || rsx!{<div>fallback</div>}),
    )
  ]
}
```

### ECS Server Actions

Thanks to our bevy router, server actions are now regular systems:

```rust
// add.rs
pub fn get(params: In<(i32, i32)>) -> i32 { 
  params.0 + params.1
}
```

### Client Island Scenes

Client islands are now regular bevy scenes embedded in the html, no more codegen! 🥳

```html
<script type="beet/client-islands">(
  ...
  components: {
    "NodeTag": ("ServerCounter"),
    "ClientLoadDirective": (),
    "DomIdx": (3),
    "ServerCounter": (
      value: (initial: 0)
    ),
  },
</script>
```

### Static files via buckets

I knew this would be required soon and guess I crossed that threshold a few days ago, lambda functions just aren't designed for serving large wasm files. These are now served via s3 alongside the ssg html.
A side benefit to this is we can update the static parts of the site via an s3 sync without needing to redeploy the server.

```rust
// crates/beet_site/src/routes.rs
fn bucket_fallback() -> impl Bundle {
	(
		HandlerConditions::fallback(),
		bucket_file_handler(),
		AsyncAction::new(async move |mut world, entity| {
			let provider = S3Provider::create().await;
			world
				.entity_mut(entity)
				.insert(Bucket::new(provider, "beet-site-bucket-prod"));
			world
		}),
	)
}
```

## Goodbye CLI

Config files are a bug not a feature, even I found the `beet.toml` hard to work with and I'm the one who wrote it! 
Previously the cli would load the config file into an tree of entities, now these are declared directly by the user, gated behind the `launch` feature.

```rust
// crates/beet_site/src/collections.rs
#[cfg(feature = "launch")]
fn pages_collection() -> impl Bundle {
	(
		RouteFileCollection {
			src: AbsPathBuf::new_workspace_rel("crates/beet_site/src/pages").unwrap(),
			..default()
		},
		ModifyRoutePath::default().base_route("/docs"),
		MetaType::new(syn::parse_quote!(beet::prelude::ArticleMeta)),
		CodegenFile::new(
			AbsPathBuf::new_workspace_rel("crates/beet_site/src/codegen/pages.rs").unwrap(),
		),
	)
}
```
### Syntax Highlighting

Last but not least we now have an integration with `syntect`, which provides all the syntax highlighting in this post :)


### Onward

And thats it for this harvest, I'll need to focus on beetmash for the next month so so the next harvest will likely be a polish release.