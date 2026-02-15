lets complete `markdown.rs` a parser in beet_stack.

We are using a diffing approach because creating entities/ inserting components etc is expensive,
and we need to consider the case where a markdown file was just changed slightly.

See `dom_diff.rs` for a similar approach, i generally prefer the SystemParam impl though this event iterator may call for another approach, use your best judgement.


The output types and should be an entity tree where the components are those found in `src/content`. we are very early into implementing a more-or-less html based layout system. you will likely need to completely refactor and add big amounts to the `src/content` directory to complete it to handle all the pulldown-cmark tag types. Consider the DisplayBlock approach, it may also need tweaking do some design work there.

see beet_stack in general for more info. `demo_stack.rs` is a good place to see the process. 

When you have completed the markdown parser, create examples/stack/demo_stack/home.md, about.md, and mod.rs in the stack directory, and ensure they are pulling it in. This is probably with a new type:

```rust
#[derive(Component)]
struct FileContent(pub WsPathBuf);
```

where its actual content gets loaded in and that entity becomes the root of the parsed tree.

happy pulling!
