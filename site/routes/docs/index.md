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
- An **app** is an instance of the engine. Running one is like pressing play on an empty editor scene: it does nothing and assumes nothing until given behavior.
- A **scene** is serializable data describing structure and behavior: UI trees, routers, behavior trees, agents, even infrastructure. BSX markup, BSN, JSON and postcard are all valid representations of the same scene.
- A **tool** is the thing a person makes and uses to change the world around them: one app or several, driven by scenes. The [embodied agent](/) on the home page is one tool made of a server, a phone and a robot.

Because behavior lives in scenes rather than compiled control flow, a tool stays open while it runs: you, your collaborators and your agents can inspect and reshape it. The slope is gentle the whole way up, from tweaking a value in a scene to extending the engine in Rust.

## One world, many interfaces

A CLI, a server and a GUI differ mostly in how bytes arrive and leave. Beet describes the tool once, as entities and components, and treats the interface as a matter of input and output. The same router serves a terminal, an HTTP request and an AI tool call; the same scene tree renders to HTML or the terminal.

## Where to go next

- [Tutorials](/docs/tutorials) walk you through building something from scratch, start here if you are new.
- [Crates](/docs/crates) explain what each piece of beet does and how they fit together.
- [Design](/docs/design) covers the target-agnostic design system.
- [About](/docs/about) tells the story behind the project and collects the reading that shapes its direction.

The [blog](/blog) follows the project's month-to-month development, and the per-crate [API docs](https://docs.rs/beet) cover the details.
