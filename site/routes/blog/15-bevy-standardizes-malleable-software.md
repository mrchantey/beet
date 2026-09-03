+++
title = "Bevy standardizes malleable software"
slug = "bevy-standardizes-malleable-software"
description = "Why I think Bevy will rule the world."
created = "2026-08-28"
author = "Pete Hayman"
+++

# Bevy standardizes malleable software

In the tradition of Bevy Birthdays we get a chance to reflect on our adventures over the past year and hopes for the future. For me the last twelve months carried two major themes:
1. Developing the storytelling of my work by presenting at [DevOps Days Wollongong](https://youtu.be/a-Sx0aEhDhc?list=PLKIKuXdn4ZMjKxet6G2oQkdIZqsKXLit7) and [Local-First Conf '26](https://youtu.be/eRpMQhOR93U?si=6SMyrZWDEOyQFphS).
2. Meeting and hearing from other people who rethinking software from first principles in adjacent fields like Decentralized Web.

Both of these endeavours have been utterly exhausting but exciting, a back-to-back DWeb Camp/Local-First adventure in Berlin has crystalized my view of the impending tech revolution and I can't wait to see Bevy's role in it unfold.

## Tech Sovereignty is ready

Martin Kleppmann headlined Local-First Conf '26 presenting the importance of ✨*Tech Sovereignty*✨ now more than ever, and over the next two days community members proposed solutions for each layer of this sovereign stack:

- **Social protocols:** ATProto/Matrix
- **Sync engines:** Automerge/PowerSync
- **Malleable software:** Patchwork/Beet

## Malleable Standards

In my last blog post, [ATProto isn't malleable yet](/blog/atproto-isnt-malleable-yet), I argued that sovereign protocols like ATProto are missing a *truely malleable* application layer. 

Over the decades we have seen many genuinely awesome solutions to malleable software but none have been standardized in the same way other technologies have. It seems interoperability is not a priority at the application layer like it is for protocols, but *it should be!* In my favorite panel discussion of all time [Robin Berjon advocates for sync standards](https://youtu.be/gjG_cUx_ueU?t=1013), another application layer problem, and similar arguments can be made for malleability.

There is plenty of discussion within the Local-First community about **data standards**, the [malleable software essay itself](https://www.inkandswitch.com/essay/malleable-software/#tools-not-apps) calls for tools that operate on shared data, not apps that silo it. The call for **malleable standards** takes this idea further: the tools themselves are data, agnostic to the client used to run them.

## Malleable Layers

Bevy's killer feature is its *layers of malleability*, each targeting a different role:

### 1. Bevy Plugins for engineers

Bevy apps are plugins all the way down. Engine internals, external libraries and business logic are all plugins registering components, systems and other plugins.

### 2. Dynamic Scripts for modders

Application code has unrestricted access to the filesystem, network and computer hardware, making it unsuitable for running untrusted code. Deno solves this at the executable level but that's a coarse instrument, permissions apply to the whole process rather than the individual scripts we may or may not trust. Lua, Rhai and QuickJS enable sandboxed scripting, but these are only as powerful as the parameters and methods an application is capable of exposing. Bevy's [Dynamic API](https://github.com/bevyengine/bevy/blob/main/examples/ecs/dynamic.rs) allow for **fine-grained exposure** of application behavior at engine, library and application levels.

### 3. Scene Files for users

Data-driven software has been the industry standard for video games since id Software's WAD files in Doom. Games are naturally multi-diciplinary and deep access for non-technical team members and modders is very valuable.

## Interoperable Scenes

Bevy's ECS architecture makes its serialized representation very simple, and basically *already interoperable* with other ECS engines. This can be seen by comparing three data-driven engines:

| Engine | Model | Component Composition | Spatial Info | Relations |
| :--- | :--- | :--- | :--- | :--- |
| **Unity** | GameObjects | ✅ Strongly typed | ❗ Required | ❗ Required and fixed |
| **Godot** | Nodes | ❗ Conventional | ✅ Optional | ❗ Required and fixed |
| **Bevy** | Entities | ✅ Strongly typed | ✅ Optional | ✅ Optional & extensible |

ECS representations need only a lightweight adaptor for interoperability with more opinionated formats. A Bevy scene can be used to describe a Godot or Unity scene, but the reverse requires lossy convention mapping.

The missing piece for true engine interoperability is standardization around components. This is the application layer equivalent of an [ATProto lexicon](https://atproto.com/specs/lexicon) and means representing components via namespaced identities extensible beyond the cargo/crates ecosystem:

```jsonc
{
  "0": {
	  // Closed: cargo/rust module path convention
  	"bevy_ecs::Name": "Billy"
    // Open: reverse domain name notation prefix
    "org.bevy.bevy_ecs.Name": "Billy"
  }
}
```

The standard specifies how components are defined but implementation is optional. Applications only implement the component definitions they care about and silently carry the rest, respecting round-trip data retention in a similar spirit to the USD spec. In this way the standard is a *method for resolving component schemas and semantics*, not the schemas themselves. This reflects how ATProto defines the spec but not the actual lexicons, BlueSky lexicons are no more standardized than any others. 

This is one area I'm excited to explore further on over the next year, perhaps by writing a scene adaptor for some other ecosystem like [Patchwork](https://www.inkandswitch.com/patchwork/notebook/). Thank you Cart, Alice and the bevy of people who created this incredible community and technological playground! 🐦
