+++
title = "Scripts"
order = 2
+++

# Scripts

A script adds behavior a scene has no words for, without recompiling anything. It is the middle layer of malleability, between editing a [scene](/docs/scenes) by hand and extending the engine with a [plugin](/docs/plugins). This page is about what a script is and what it may reach. [Teach it new words](/docs/tutorials/guestbook-scripts) writes several.

## A script is a scene

A script is not a second kind of file. It is a scene of one `RunScript` entity with its source as a child, so it is stored, shared, edited and versioned exactly as any other record, and a route that is a script is one tag:

```jsx
<ScriptRoute path="greet" script="'hello ' + (input.params.name ? input.params.name[0] : 'stranger')"/>
```

Every script is an async function body. It receives its `input` and what it returns is its answer, a string as plain text and anything else as JSON. An event on a page, `bx:click` for one, is a script of the same kind.

## No authority of its own

A script runs in QuickJS with nothing of the host to reach for. It has no filesystem, no network and no environment, and it never holds the world. It reaches the world only by asking, one call at a time, over a channel the host serves and can refuse:

```js
const found = await world.entities('guestbook.Visits');
const counter = found.length ? found[0] : await world.spawn({ 'guestbook.Visits': 0 });
await world.insert(counter, 'guestbook.Visits', (await world.get(counter, 'guestbook.Visits')) + 1);
```

`ScriptConfig` names what a script may read and write, by component name or by pattern, and the bridge enforces it rather than the script. A refused call arrives as an error where the call was made, so the script can catch it and carry on, and `world:false` withholds the world entirely, leaving a script that is provably a pure transform of its input. This is what lets a script be untrusted and useful at the same time, and why a script you did not write gets a shorter list.

## New words

A scene can mint a component type the engine never shipped:

```jsx
<DynamicComponent name="guestbook.Visits" schema="u64"/>
```

The declaration is part of the scene, so a saved and reloaded scene still has the word. The schema is its meaning, so a write of the wrong shape is refused the same way a write outside the grant is, where the call was made.

## Where the layer ends

A script cannot hold authority, and durable storage, a network connection and a device are authority. When a scene needs one of those, the word for it is compiled into a [plugin](/docs/plugins), and the scene addresses it with the same tag it would use for any other component.
