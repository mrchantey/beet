+++
title = "Glossary"
order = 6
+++

# Glossary

Below is a list of definitions from the perspective of the beet project.

- **Account.** An atproto identity beet acts as: a did, its handle, the PDS hosting its repo, and optionally a credential to write with. Declared once in an entry document and named from anywhere in it.
- **Action.** An entity that behaves like a function, with one action per entity. How behavior is spelled in a scene, from a route handler to a behavior tree node to a deploy step. A **provider** is a component whose requirements supply the entity's action, and an **overload** is an extra signature on the same action, which is how one entity answers a request, a behavior tree and a script without a second action.
- **Actor.** A participant in a thread, a person, a prompt or a model agent on equal footing, an entity whose turn is an action.
- **Announcement.** The Bluesky post introducing one standard site document, carrying the canonical url and strong references to the document and its publication. Written once and never edited, so its replies can be the article's comments.
- **Atmosphere.** The ecosystem of people, developers, applications and tools built around the [AT Protocol](https://atproto.com/). Beet is designed for it and runs without it, so a repo on a plain directory today is the same repo on a PDS later.
- **Blob.** Bytes with a mime type and a path, opaque to beet, replaced whole and never edited in place. Every file in a branch is a blob, keyed by its path with the extension kept: a binary or a wasm build is a blob like an image is, and so is a document, which is a blob beet knows how to read.
- **Branch.** A package's mutable working files, the thing an author edits. Every package has a `main` branch, and a new branch starts from the files of the one it came from and replaces only what it writes. Nothing compiled lives in a branch.
- **Deploy version.** The versioned copy a deploy publishes, recorded in the ledger. Which blobs are versioned per deploy is per-blob or per-directory metadata.
- **Derived file.** A file computed from declared sources and stored beside the provenance naming them, regenerated when stale or missing and read by consumers that never ask how it was made. One that cannot be reproduced from its sources alone is committed under the package's `derived/` directory so nobody mistakes it for a source; a reproducible one is a cache.
- **Document.** A file whose media type beet can read, and the same content once it is loaded. A **file document** is the bytes in a branch; a **runtime document** is that content loaded as a value and projected onto entities, the thing widgets bind to and edit. Markdown, BSX, JSON and an Automerge file are documents; a png is only a blob.
- **Entry document.** The one document a package runs from, `main.bsx` at its root, the data driven entry point to the program. It carries the package declaration, and servers, routes, behaviors and deploys are entities declared in it or in the documents it pulls in. A runtime finds it by walking up from the current directory, or is pointed at one with `--main`.
- **Fork.** A replica of a repo you do not own, holding local edits that shadow upstream until discarded. A state of the replica, not an edit.
- **Home directory.** An author's local canonical files, the working tree, under git or not. Pushed into a repo, never the repo itself.
- **Homegrown tech.** Tech created in place by the person who uses it, growing as their requirements evolve, in the lineage of [home-cooked apps](https://www.robinsloan.com/notes/home-cooked-app/) and [folk technology](/blog/folk-technology). The contrast is with software built elsewhere, suiting most people most of the time and nobody all of the time, and with app as short for application specific software designed for a narrow purpose.
- **Instance.** A reference from one document to another with named arguments, and the subtree it produces: a clone of the target, carrying a mark so its origin is known.
- **Ledger.** The deploy's own record of what was published when, which stage it targeted and which deploy version it published.
- **Package.** A directory holding an entry document, the unit of compilation, dependency and versioning, and the one a process runs from. Packages nest, and a repo's root directory is a package, its default.
- **Package record.** One published version of a package: immutable, naming the branch it was compiled from, each document's snapshot, each blob, and what a loader and a resolver need. Listing them is the version list.
- **Plugin.** Compiled Rust extending the engine, shared as a crate. The deepest layer of malleability and the only one that holds authority. See [Plugins](/docs/plugins).
- **Publication.** A [standard.site](https://standard.site) record describing a site that publishes documents: its url, name, description, icon and theme. The beet blog is one, declared in the entry document and converged onto the account's repo.
- **Record.** An entity stored in a repo under a collection and a key, in the [AT Protocol](https://atproto.com/) sense: a small body of components keyed by schema name, addressed by `at://<did>/<collection>/<key>` and pinned by content id. Beet's own records are branches, package records and snapshots; a repo holds foreign records beside them.
- **Replica.** A device's local copy of a repo, what a process reads and writes.
- **Repo.** An account's whole tree, its documents, blobs and records, one to one with the account's PDS repo when published and a directory or any other store backend otherwise. The account and the repo are one unit, so the qualifier "account repo" adds nothing.
- **Route.** Usually a document that also carries a path, sometimes a few entities with a path and an action. The url space is a projection over the repo, and a `Router` roots one.
- **Runtime.** A binary that runs packages from their main document. The `beet` cli is one, a crate adding `BeetPlugins` and its own plugins is another, and a wasm build of either is what a browser tab fetches. The entry document declares the runtime it needs and where a browser can fetch it, which a browser requires and a native binary ignores, running what it has.
- **Scene.** An entity graph as data, hence JSON-shaped, hence mergeable. Beet's unit of editing, at the component grain. A document compiles into one and a tool can author one directly, so a scene is the shape and never the file. See [Scenes](/docs/scenes).
- **Script.** A scene of one `RunScript` entity with its source as a child, run sandboxed in QuickJS under a `ScriptConfig` naming what it may read and write. The middle layer of malleability. See [Scripts](/docs/scripts).
- **Snapshot.** One document compiled into a scene, and what a runtime actually loads. Addressed by the content it holds, so two versions of a package whose document did not change share one snapshot. Every snapshot is a scene; not every scene is a snapshot.
- **Stage.** The environment a deploy targets, `dev` or `prod` for one.
- **Standard site document.** The [standard.site](https://standard.site) record for one published post: its title, description, dates, tags, contributors, and the path that joins the publication's url to form the canonical page. Derived from the post and never authored by hand. The qualifier is always written, since a bare document is beet's own word for a file beet can read.
- **Swappable stack.** Every layer beet stands on, the renderer, the http server, the blob store and the filesystem among them, is an interface with more than one implementation and none privileged. In the spirit of Bevy's plugins and atproto's credible exit.
- **Template.** Runtime only. The trait and its `#[template]` constructors, compiled Rust that expands into entities when a document builds. Never a file.
- **Thread.** A conversation as a scene of actors with a transcript, one shared shape for human conversations and agent harnesses.
- **Upstream.** A repo's canonical location, a served mount today, a bucket or a PDS later.
- **App.** Short for application software, a legacy mindset that software is a fixed thing that must be designed for a specific purpose, see [Tools, not apps](https://www.inkandswitch.com/essay/malleable-software/#tools-not-apps).
