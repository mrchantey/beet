+++
title = "beet_scene"

[RouteMeta]
visibility = "unlisted"
+++

# beet_scene

`beet_scene` turns documents into scenes, and instantiates scenes within one another.

A document is what a person writes: BSX, HTML, markdown, JSON, TOML, plain text or javascript. Each format has one parser, a `DocParser`, and every parser does the same thing: bytes in, a scene body out, touching no world. Everything after the body is one pipeline that knows nothing of the format it came from.

```rust
# use beet_core::prelude::*;
# use beet_scene::prelude::*;
let mut world = ScenePlugin::world();
let bytes = MediaBytes::new_bsx("<main><p>hello</p></main>");
let mut entity = world.spawn_empty();
ParseContext::new(&mut entity, &bytes).parse().unwrap();
assert_eq!(entity.get::<Element>().unwrap().tag(), "main");
```

A body goes both ways. `TemplateLoader` builds one into a world, `TemplateSaver` writes a world's subtree back as one, and `SceneDocument` holds a loaded scene as an editable document the world follows.

One document uses another by naming it, `<Instance document="widgets/Card.bsx" props={{title:"Intro"}}/>`. The document is read once and held as data, and each use builds it with entities of its own, its props and slots filled by whoever uses it.

The crate is `no_std`, so firmware loads a scene pushed to it. What names the data and nothing more, the scene body, its keys and schemas, the format registry and the template substrate, is [`beet_core`](/docs/crates/beet_core)'s, so a crate can declare a type that any document may use without linking a parser.
