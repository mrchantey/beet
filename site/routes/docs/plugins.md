+++
title = "Plugins"
order = 3
+++

# Plugins

A plugin extends the engine itself. It is the deepest layer of malleability, the one that is allowed to hold authority, and it is written in compiled Rust. This page is about what a plugin is and why writing one is the same act as building with beet. [Extend the engine](/docs/tutorials/guestbook-plugins) writes one.

## Plugins all the way down

A beet runtime is [Bevy](https://bevy.org) plugins all the way down. Engine internals, libraries and your own code all take the same shape, a `Plugin` that registers components, systems and other plugins. `BeetPlugins` is the library of capabilities the `beet` binary links, and a plugin of yours sits beside it:

```rust
App::new()
	.add_plugins(BeetPlugins)
	.add_plugins(GuestbookPlugin)
	.run()
```

Since no layer is privileged, a plugin that swaps the http server or the renderer looks exactly like one that adds a guestbook action, and extending beet is the same act as building with it.

## Words a scene can use

What a plugin contributes is vocabulary. A registered reflect type is a tag a [scene](/docs/scenes) may author, so a compiled action becomes one line of markup:

```rust
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
async fn SignGuestbook(cx: ActionContext<Entry>) -> Result<String> {
	// ..
}
```

```jsx
<Route path="guests/sign" {SignGuestbook}/>
```

Where the action lives in the url space is the scene's decision rather than the plugin's, so a compiled word is moved, renamed and composed without a rebuild. A runtime lacking the plugin leaves the tag inert rather than failing, so the same scene loads whole in a lean binary and in a full one.

## The layer with authority

[Scripts](/docs/scripts) hold no authority by design. Durable storage, the network and the hardware are authority, so the words for them are compiled. Beet's own stores, servers and renderers are plugins of this kind, and the swappable stack in the [principles](/docs/about) is what it looks like when every one of them has more than one implementation.

## Runtimes

A plugin reaches a person through a runtime, a binary that links it. The `beet` cli is the runtime with no plugins of yours, a crate that adds `BeetPlugins` and one of yours is another, and a wasm build of either is what a browser tab fetches. The main scene names the runtime it needs, so the entry comes first and the binary follows from it.
