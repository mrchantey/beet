## Beet By Features

### Infra As Code

Our `beet_infra` SST template deploys an entire stack including a cargo lambda, sqlite database, custom domain and s3 bucket. SST is very accessible infra as code, enabling stack deployment in minutes while maintaining 100% control.

```sh
cargo binstall beet cargo-generate
cargo generate beet_todo
cd beet_todo
# if aws credentials are configured this will deploy an app
beet deploy --sst
# ..aaand we're live!
beet remove --sst
# all resources cleaned up :)
```

### ORM Integration

Make seamless database calls by combining the Diesel ORM with server actions.

<div server:defer>{Foo::num_entries().rx()}</div>
<Button onclick={|_|Foo::add_default()}</Button>

### Rusty Markdown

Take your docs to the next level with rust and rsx components in your markdown documents.

<Tabs>
<TabItem>
*This paragraph was written in markdown, inside a `<Tabs>` component.*
</TabItem>
<TabItem>
*So was this one!*
</TabItem>
</Tabs>

### Signals

The lightweight and battle hardened leptos signals crate provides fine-grained reactivity on both the client and server.

<Counter/>

## Live reload

Changes to the non-code part of rsx will cause instant rebuilding of the html files without a recompilation required.

### Islands Architecture

Every beet page is static by default, using `<Counter client:load/>` to load interactive components, and `<Avatar server:defer/>` to load components asynchronously from the server.

### Component scoped styles

Use the `<style>` tags in a component to apply styles only to its elements, and customize this behavior with directives like `scope:global` and `style:cascade`. Styles are automatically minified and deduplicated.

### Server Signals & Actions

Push changes to the client with server signals, make typesafe API calls with server ations, and combine them with `beet_query` to create a sync engine.

### Design System

Our component library, color calculator and themes provide beautiful defaults and deep customization.

### And many more

- Slots
- Prehydration Event Playback