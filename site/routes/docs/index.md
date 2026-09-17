+++
title = "Docs"
order = 0
expanded = true
+++

# Beet

Beet is an operating system built around a person rather than a machine. The software you build on it is a repo of scenes, records and blobs that belongs to an account, and can be accessed via browser, terminal, ssh session or native GUI.

> 🚧 Beet is pre-release and under active construction. If it sounds interesting, come and say hi in the [Beet Roomy space](https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm).

## How it works

Beet is built on the [Bevy](https://bevy.org) game engine, and everything in beet, from a ui tree to a router to a cloud deploy, is Entity Component System (ECS) data. A handful of words carry the model.

- A **repo** is an account's whole store, its scenes, other records and blobs. It is the thing a runtime runs and the thing that belongs to you.
- A **scene** is a record describing structure and behavior as an entity graph, so it is JSON-shaped and mergeable, and it is beet's unit of editing. BSX markup, markdown and JSON are all ways of writing one.
- A **blob** is bytes with a mime type and a path, an image, a wasm build, a binary. Beet replaces a blob whole and never edits one in place.
- The **main scene** is the entry point, `main.bsx` at the root of a repo, the data driven entry point to the program. Servers, routes, behaviors and deploys are entities declared in it or in the scenes it pulls in.
- A **runtime** is a binary that runs repos. The `beet` cli is one, and it does nothing until given a main scene, like pressing play on an empty editor scene. The main scene names the runtime it needs and where a browser can fetch one.
- An **action** is an entity that behaves like a function, and is how behavior is spelled in a scene, from a route handler to a behavior tree node to a deploy step.

Because behavior lives in scenes rather than compiled control flow, software stays open while it runs, ready for you, your collaborators and your agents to inspect and reshape.

## Malleable

Malleability in beet is a slope rather than a cliff, with a real mechanism at every level of involvement, and each layer stays open while the software runs.

- **Scenes** reshape software as it runs. Since behavior is data, changing a thing means editing a scene, in a text editor, in the scene editor on any surface, or through an agent, with no fork, no rebuild and no redeploy.
- **Scripts** add behavior a scene has no words for. A script is itself a scene, one `RunScript` entity with its source as a child, that runs in QuickJS under declared resource ceilings with no ambient authority, reaching the world only by asking, one call at a time, over a channel the host serves and can refuse.
- **Plugins** extend the engine itself. A beet runtime is plugins all the way down, with engine internals, libraries and your own code all taking the same shape, so extending beet is the same act as building with it.

A person tweaking a value in a scene today can reach for a script tomorrow and a plugin after that, without the thing they made ever being thrown away and rewritten.

## Agnostic

A runtime does nothing by default and assumes nothing about what you are making, and everything it stands on is a swappable stack.

- **Surfaces.** The same scene renders to the DOM, to a terminal grid, to html, markdown and plaintext, and drives a windowed Bevy app. A binary lacking a capability leaves those components inert rather than failing.
- **Domains.** The same capabilities serve web sites, games, robots, infra deploys and agent harnesses, since each is a scene of actions.
- **The stack.** The http server, the renderer, the blob store and the filesystem are interfaces with more than one implementation and none privileged, so a server runs on wasm and a terminal ui runs over ssh. Keep what you love and swap out the rest, in the spirit of Bevy's plugins and atproto's credible exit.

## Decentralized

A repo belongs to an account and runs the same everywhere. Identity and sync sit above it as capabilities the repo does not depend on, atproto first.

- **Your account.** Your software is records and blobs in a repo, and an atproto PDS is its home once sync lands, one account among many.
- **Local-first.** A repo works with no network at all, on a plain directory or in a browser's storage, and syncs when a network appears.
- **Honest formats.** Import is generous, so markdown, html, BSX or a whole directory come in, and export is honest, giving back the format you asked for without promising byte identity. A git repo can stay the source of truth for a large project, pushed into the repo one way.
- **An open format.** The [scene format](/docs/scene-format) is designed to outgrow beet, so other software can read what you made.

## Where to go next

- [Tutorials](/docs/tutorials) build one guestbook three times over, once at each layer of malleability. Start here if you are new.
- The [Scene Format](/docs/scene-format) is the draft standard beet implements, and where it is going.
- [Crates](/docs/crates) explain what each piece of beet does and how they fit together.
- [Design](/docs/design) covers the target-agnostic design system.
- [About](/docs/about) tells the story behind the project and collects the reading that shapes its direction.

The [blog](/blog) follows the project's month-to-month development, and the per-crate [API docs](https://docs.rs/beet) cover the details.
