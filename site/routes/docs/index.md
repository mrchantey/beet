+++
title = "Docs"
order = 0
expanded = true
+++

# Beet

Beet is a malleable engine built on interoperable standards, with apps driven by json scene files that users own and edit. Beet operates at the application layer of the sovereign stack, complementing DWeb and local-first technologies with standardized malleable software.

> 🚧 Beet is pre-release and under active construction. If it sounds interesting, come and say hi in the [Beet Roomy space](https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm).

## How it works

Beet is built on the [Bevy](https://bevy.org) game engine, and everything in beet, from a UI tree to a router to cloud infrastructure, is Entity Component System (ECS) data. Four words carry the model:

- An **engine** is a library of capabilities with no prescribed use. Beet is an engine, as is Bevy beneath it.
- An **app** is an instance of the engine. Running one is like pressing play on an empty editor scene, doing nothing and assuming nothing until given behavior.
- A **scene** is serializable data describing structure and behavior, ie UI trees, routers, behavior trees, agents, even infrastructure. BSX markup, BSN, json and postcard are all representations of the same scene.
- A **tool** is a scene that defines actions to perform on data, the thing a person makes and uses to change the world around them.

Because behavior lives in scenes rather than compiled control flow, a tool stays open while it runs, ready for you, your collaborators and your agents to inspect and reshape.

## Malleable at every layer

Malleability in beet is a slope rather than a cliff, with a real mechanism at every level of involvement.

1. **Plugins** extend the engine itself. A beet app is plugins all the way down, with engine internals, libraries and business logic all taking the same shape, so extending beet is the same act as building with it.
2. **Sandboxed scripts** add behavior a scene has no words for. A script runs in QuickJS under declared resource ceilings with no ambient authority, reaching the world only by asking, one call at a time, over a channel the host serves and can refuse. A scene narrows that reach per script, by naming components or by pattern. The same engine serves native, browser and microcontroller environments.
3. **Scene files** reshape a tool while it runs. Since behavior is data, changing a tool means editing a scene, with no fork, no rebuild and no redeploy.

The slope is gentle the whole way up, so a person tweaking a value in a scene today can reach for a script tomorrow and a plugin after that, without the tool ever being thrown away and rewritten.

## An open format

The scene format is designed to outgrow beet. The [Scene Format](/docs/scene-format) page records its current state, which is currently just bevy scenes, and the path toward a standard any engine can implement.

## Where to go next

- [Tutorials](/docs/tutorials) build one guestbook three times over, once at each layer of malleability. Start here if you are new.
- The [Scene Format](/docs/scene-format) is the draft standard beet implements, and where it is going.
- [Crates](/docs/crates) explain what each piece of beet does and how they fit together.
- [Design](/docs/design) covers the target-agnostic design system.
- [About](/docs/about) tells the story behind the project and collects the reading that shapes its direction.

The [blog](/blog) follows the project's month-to-month development, and the per-crate [API docs](https://docs.rs/beet) cover the details.
