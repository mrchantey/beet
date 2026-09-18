+++
title = "Glossary"
order = 6
+++

# Glossary

Below is a list of definitions from the perspective of the beet project.

- **Action.** An entity that behaves like a function, with one action per entity. How behavior is spelled in a scene, from a route handler to a behavior tree node to a deploy step.
- **Atmosphere.** The ecosystem of people, developers, applications and tools built around the [AT Protocol](https://atproto.com/). Beet is designed for it and runs without it, so a repo on a plain directory today is the same repo on a PDS later.
- **Blob.** Bytes with a mime type and a path, opaque to beet, replaced whole and never edited in place. A binary or a wasm build is a blob like an image is.
- **Deploy version.** The versioned copy a deploy publishes, recorded in the ledger. Which blobs are versioned per deploy is per-blob or per-directory metadata.
- **Document.** Runtime only. The parentless value a record materializes as, the thing widgets bind to. Never a file on disk.
- **Fork.** A replica of a repo you do not own, holding local edits that shadow upstream until discarded. A state of the replica, not an edit.
- **Home directory.** An author's local canonical files, the working tree, under git or not. Pushed into a repo, never the repo itself.
- **Homegrown tech.** Tech created in place by the person who uses it, growing as their requirements evolve, in the lineage of [home-cooked apps](https://www.robinsloan.com/notes/home-cooked-app/) and [folk technology](/blog/folk-technology). The contrast is with software built elsewhere, suiting most people most of the time and nobody all of the time, and with app as short for application specific software designed for a narrow purpose.
- **Instance.** A subtree cloned from a named reusable scene, carrying a mark so its origin is known.
- **Ledger.** The deploy's own record of what was published when, which stage it targeted and which deploy version it published.
- **Main scene.** The entry point of a repo, `main.bsx` at its root, the data driven entry point to the program. Servers, routes, behaviors and deploys are entities declared in it or in the scenes it pulls in. A runtime finds it by walking up from the current directory, or is pointed at one with `--main`.
- **Plugin.** Compiled Rust extending the engine, shared as a crate. The deepest layer of malleability and the only one that holds authority. See [Plugins](/docs/plugins).
- **Record.** A scene or a document stored in a repo, JSON today, keyed by a name. The unit of sync and of lazy loading.
- **Replica.** A device's local copy of a repo, what a process reads and writes.
- **Repo.** An account's whole store, its scenes, other records and blobs. The one store a process runs from, marked `RepoStore`, one per world.
- **Route.** Usually a scene that also carries a path, sometimes a few entities with a path and an action. The url space is a projection over the repo, and a `Router` roots one.
- **Runtime.** A binary that runs repos. The `beet` cli is one, a crate adding `BeetPlugins` and its own plugins is another, and a wasm build of either is what a browser tab fetches. The main scene declares the runtime it needs and where a browser can fetch it, which a browser requires and a native binary ignores, running what it has.
- **Scene.** A record describing structure and behavior as an entity graph, hence JSON-shaped, hence mergeable. Beet's unit of editing, at the component grain. See [Scenes](/docs/scenes).
- **Script.** A scene of one `RunScript` entity with its source as a child, run sandboxed in QuickJS under a `ScriptConfig` naming what it may read and write. The middle layer of malleability. See [Scripts](/docs/scripts).
- **Stage.** The environment a deploy targets, `dev` or `prod` for one.
- **Swappable stack.** Every layer beet stands on, the renderer, the http server, the blob store and the filesystem among them, is an interface with more than one implementation and none privileged. In the spirit of Bevy's plugins and atproto's credible exit.
- **Template.** Runtime only. The trait and its `#[template]` constructors, compiled Rust that expands into entities when a scene builds. Never a file.
- **Upstream.** A repo's canonical location, a served mount today, a bucket or a PDS later.
- **App.** Short for application software, a legacy mindset that software is a fixed thing that must be designed for a specific purpose, see [Tools, not apps](https://www.inkandswitch.com/essay/malleable-software/#tools-not-apps).
