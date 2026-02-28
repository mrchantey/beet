# Beet Stack

Lets keep iterating on beet_stack!

continue with the feature gating of beet_stack. as you can see from current unstaged git diff markdown is now feature gated, and we need to update all tests to use mime_load_tool and mime_render_tool, 

if a test expects rendered markdown, use Request::with_header(Accept,MimeType::Markdown) (may need to add the helper methods to beet_net)

the mime_render_tool and mime_load_tool json and postcard impls were incorrectly using markdown when they should be using scene formats. the tests are likely incorrect.

when loading scenes from the MimeLoadTool we'll need to remove all children from the entity, and reparent all roots of the spawned scene under it. this spawn behavir should be upstreamed into the SceneLoader itself. 

```rust
struct SceneLoader {
	..
	// if set, loads all spawned root entities as children
	// of this entity
	entity: Option<Entity>
}
```

We can use the EntityHashMap values which is a list of spawned entities, iterate all and filter by those without a ChildOf.


### Testing


use these tests to see the state and work that still needs doing
cargo test -p beet_stack --lib | tail
cargo test -p beet_stack --all-features | tail
cargo test -p beet_stack --lib --all-features --target=wasm32-unknown-unknown | tail
