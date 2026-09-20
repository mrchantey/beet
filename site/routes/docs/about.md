+++
title = "About"
order = 7
+++

# About

Beet is an engine for the act of creation. A student learning to code, a company delivering value and a person building tools to manage their own life are all doing the same thing, and beet's job is to serve them all with one broad, deep stack.

## Principles

Wiring atproto into existing software is has some benefits but what makes the integration meaningful is that the software on the other side already lives by the protocol's values, so that a person can leave with their data, run it somewhere else, reshape it without asking, and find their schemas understood by the next tool along. Beet is built on three principles that hold whether or not a network is present, and the atproto integration is what they were always pointing at. That is what Atmospheric means here, in reference to the Atmosphere, the ecosystem around atproto, and it holds for a repo that has never seen a network.

The three are orthogonal. Malleable answers how software changes, agnostic answers where it runs, and decentralized answers whose it is and where it lives. The [home page](/) states each, and these are the working rules under them.

**Malleable.** The scene is the only thing beet works from, since text formats, markdown, HTML and BSX, are for importing, exporting and talking to people and models, and none is ever authoritative because text is hard to parse and easy to lose structure in. Editing happens where you are looking, on any surface, with no filesystem, IDE or git required. Seed once, then live: an existing tree of files is the seed of a repo, and after the import the repo is the truth and the tree is history unless the author keeps pushing it.

**Agnostic.** A runtime does nothing until given a main scene, so running one is like pressing play on an empty editor scene, and every server, route and behavior is declared in the scene and never assumed by the binary. Every layer of the stack is an interface with more than one implementation and none privileged, in the spirit of Bevy's plugins and atproto's credible exit. The main scene comes first and the binary follows, naming the runtime it needs and where a surface can fetch one, while a native surface runs the binary it has and leaves the components it lacks inert.

**Decentralized.** A repo is scenes, records and blobs, no exceptions, with blobs as opaque bytes in one known place and nothing half a scene. Import is generous and export is honest, so anything with a loader comes in, what goes out is the format you asked for, byte identity is never promised, and an author who keeps a home directory as their truth pushes it into their repo one way. A repo belongs to an account and runs the same everywhere, canonical on the account's PDS once sync exists and a plain directory until then.

**Built with beet.** beet.org is one repo among many, edited and published through the same tools any repo uses. A workflow that cannot publish beet.org is unfinished, and a workflow beet.org does not use is not the primary one.

## The name

Beet is short for Beetmash, a mispronunciation of Beat Match which nods to [MIT Scratch](https://scratch.mit.edu) continuing the DJ theme.

## History

1. Beet started as an entities-as-behavior library.
2. In need of a framework for presenting the beet examples, beet evolved into a web meta framework, something like Astro but written in Rust.
3. It then gained all kinds of other scene loading capabilities and other abstractions, like blob stores, and became a general purpose malleable engine.
4. Finally, it gained at proto integrations and developed into an Atmospheric OS for homegrown tech.

## Beet and beetmash

Beet is the open source engine and standards and [Beetmash](https://beetmash.com) is the consulting company built on top, helping organizations toward tech sovereignty. Beetmash is to Beet roughly what BlueSky is to ATProto.

## The lineage

The ideas behind beet are not new, and it stands in a long line of tools and thinkers:

- **Seymour Papert and constructionism**: learning is itself an act of creation, we come to understand the world by making things in it. Logo, Lego Mindstorms and [Scratch](https://scratch.mit.edu) brought this to students, and their spirit runs through beet.
- **Smalltalk and HyperCard**: software as something people reshape, not just run. A beet tool keeps its structure and behavior open as data for the same reason.
- **[Malleable software](https://www.inkandswitch.com/essay/malleable-software/)**: Ink & Switch's essay names the qualities beet is built around, above all the gentle slope from user to creator.
- **Identity-native systems**: Urbit and Solid put a person's software and data under their own identity. Beet takes the same aim on an open protocol and a mainstream engine, and runs with neither present.
- **Game engines**: the tradition of separating capability from behavior, an executable driven entirely by scene files. Beet is built on [Bevy](https://bevy.org), the engine that carries this tradition from microcontrollers to the web.

The reading behind these ideas is collected below, and the [blog](/blog) follows the journey month by month.

## References

Malleable software, learning through making, and simulation as a medium all have a long lineage, and the work below shapes the project's direction.

### Malleable software

The direct lineage, software as a medium people reshape rather than consume.

- 1967 - Marshall McLuhan - The Medium Is The Massage
	- [youtube](https://youtu.be/cFwVCHkL-JU)
	- [pdf](https://designopendata.wordpress.com/wp-content/uploads/2014/05/themediumisthemassage_marshallmcluhan_quentinfiore.pdf)
- 2024 - Maggie Appleton - Home-Cooked Software and Barefoot Developers
	- [youtube](https://youtu.be/qo5m92-9_QI)
	- [blog](https://maggieappleton.com/home-cooked-software)
- 2025 - Bryan Cantrill - The Complexity of Simplicity
	- [youtube](https://youtu.be/Cum5uN2634o)

## Decentralized Tech

- [Urbit](https://urbit.org/)
	- [Article - Can Urbit reboot computing?](https://reason.com/2016/06/21/can-urbit-transform-the-internet/)
- [Solid Project](https://solidproject.org/)

### Education

Papert pioneered the higher level ideas behind malleability, that we come to understand the world by making things in it, and these pieces carry that thread into modern STEM practice.

- [Summary - Mindstorms](https://medium.com/bits-and-behavior/mindstorms-what-did-papert-argue-and-what-does-it-mean-for-learning-and-education-c8324b58aca4)
- [Article - STEM beyond robotics](https://www.theeducatoronline.com/k12/news/stem-education-must-go-beyond-focus-on-robotics-and-coding/284841)
- [Research - Promoting engagement in STEM Australia](https://research.acer.edu.au/cgi/viewcontent.cgi?article=1285&context=research_conference)

### Simulation

Simulation as a medium for understanding, the kind of tool beet wants to make easy to build and share.

- [The Washington Post - Social Distancing Sim](https://www.washingtonpost.com/graphics/2020/world/corona-simulator/)
- [Parable of the Polygons](https://ncase.me/polygons/)
