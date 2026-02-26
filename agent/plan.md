Lets keep iterating on headers, ie our new `header_map.rs`

- Entirely remove `exchange_format.rs`, users should use `mime_serde` directly.

- headers should be typed wherever possible.

Move header_map::ContentType and Accept into an adjacent module, not reexported by default.

```
// ./header_types.rs

Accept..
ContentType..


// mod.rs
pub mod headers;

// usage

request.headers.get(headers::ContentType)..
```

Add all the common ones, and update existing ones to handle missing types, ie `text/event-stream`: MimeType::EventStream


We should very rarely see raw usage of the header map in the codebase, ie `headers.get_raw`