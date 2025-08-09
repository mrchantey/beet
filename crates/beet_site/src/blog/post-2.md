+++
title = "The Full Moon Harvest #2"
+++

The stack bevyfication continues this month, with ecs continuing to prove itsself as a paradigm general enough to represent anything!

## A very bevy router

`0.0.7` marks the first release for a very bevy router:

- Simple extractors and response types (json, query params, path)

```rust

fn routes() -> impl Bundle{
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



Axum was designed for extreme performance use-cases

- bevy router
- bevy server actions
- bevy client islands
- syntax highlighting