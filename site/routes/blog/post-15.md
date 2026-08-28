+++
title="Bevy standardizes malleable software"
created="2026-08-28"
+++

# Bevy standardizes malleable software

In the tradition of Bevy Birthdays we get a chance to reflect on our journeys. For me the last twelve months carried two major highlights:
1. Presenting at [DevOps Days Wollongong](https://youtu.be/a-Sx0aEhDhc?list=PLKIKuXdn4ZMjKxet6G2oQkdIZqsKXLit7) and [Local-First Conf '26](https://youtu.be/eRpMQhOR93U?si=6SMyrZWDEOyQFphS) was an opportunity to develop the storytelling and positioning of my work.
2. Meeting and hearing from people in adjacent fields like the Decentralized Web who are also rethinking software from first principles.

Both of these endeavours have been utterly exhausting but exciting. My back-to-back DWeb Camp/Local-First adventure in Berlin has crystalized my view of the impending tech revolution and I can't wait to see Bevy's role in it unfold.

## Tech Sovereignty is ready

Martin Kleppmann headlined Local-First Conf '26 presenting the importance of ✨*Tech Sovereignty*✨ now more than ever, and over the next two days community members presented their solutions at each layer of the sovereign stack:

- **Peer-to-peer:** Iroh
- **Social web:** ATProto
- **Local-first:** Automerge
- **Malleable software:** Bevy?

## Malleable Standards

In my last blog post, [ATProto isn't malleable yet](/blog/posts/post-14), I proposed that sovereign protocols like ATProto are missing a malleable application layer. Over the decades we have seen many genuinely awesome solutions to malleable software, but none have been standardized in the same way other technologies have.

At a high level I believe this is because application developers don't think about interoperability in the same way networking folks do, and this is a natural consequence of the kinds of problems we're solving. A narrow siloed app still works for that use case, whereas an isolated node on a bespoke protocol is useless. Interoperability is not inherent to applications but *it should be!* In my favorite panel discussion of all time [Robin Berjon underscores the importance of commoditization](https://youtu.be/gjG_cUx_ueU?t=1013) at the application layer.

There is plenty of discussion within the Local-First community about **data standards**, the [malleable software essay itself](https://www.inkandswitch.com/essay/malleable-software/#tools-not-apps) calls for tools that operate on shared data, not apps that silo it. The call for **malleable standards** takes this idea further: the apps themselves are data-driven and the tools are client agnostic.

## Malleable Layers

The reason I quit a comfortable job at PlaySide Studios to hack on Bevy full time is that I believe Bevy is the most malleable technology and the future of the application layer, and after three years I still feel like I haven't even begun to understand the implications of that.

Concretely Bevy is unique in that it has hypercard-like layers of malleability, each targeting a different role:

### 1. Engineers: Bevy Plugins

Bevy apps are plugins all the way down. Engine internals, external libraries and business logic are all plugins, creating an unprecedented level of interoperability in application code.

### 2. Tinkerers: Sandboxed Scripts

Application code has unrestricted access to the filesystem, network and computer hardware. Deno solves this at the executable level but that's a coarse instrument, permissions apply to the whole process rather than the individual scripts we may or may not trust. Lua, Rhai and QuickJS enable sandboxed scripting, but these are only as powerful as the parameters and methods an application is capable of exposing. Bevy's built-in [dynamic capabilities](https://github.com/bevyengine/bevy/blob/main/examples/ecs/dynamic.rs) allow for **fine-grained exposure** of application behavior at engine, library and application levels.

### 3. Users: Scene Files

Data-driven software has been the industry standard for video games since id Software's WAD files in Doom. Where the above two layers are the result of an elegant implementation of ECS, the scene format is pure ECS. In other words, Bevy's scene format is *already interoperable* with other ECS engines.

The clearest way to understand this is by comparing three data-driven engines:

| Engine | Model | Component Composition | Spatial Info | Relations |
| :--- | :--- | :--- | :--- | :--- |
| **Unity** | GameObjects | ✅ Strongly typed | 👎 Required | 👎 Required and fixed |
| **Godot** | Nodes | 👎 Convention-based | ✅ Optional | 👎 Required and fixed |
| **Bevy** | Entities | ✅ Strongly typed | ✅ Optional | ✅ Optional & extensible |

ECS representations need only a lightweight adaptor for interoperability with more opinionated formats. A Bevy scene can be used to describe a Godot or Unity scene, but the reverse requires lossy convention mapping.

## Shared Component Definitions

The missing piece from true engine interoperability is standardization around components. This is the application layer equivalent of an [ATProto lexicon](https://atproto.com/specs/lexicon) and means representing components via namespaced identities extensible beyond the cargo/crates ecosystem:

```jsonc
{
  "0": {
    // Interoperability Example:
    // Reverse domain name notation prefix
    "org.bevy.bevy_ecs::Name": "Billy"
  }
}
```

This follows a similar pattern to how the HTML spec defines a `<details>` element and browsers implement accordingly. Where this aligns more closely with ATProto lexicons than WHATWG standards is in decentralization. The standard specifies how components are defined but implementation is optional. Applications only implement the component definitions they care about and silently carry the rest, respecting round-trip data retention in a similar spirit to the USD spec. 

Application level standardization is something I'm excited to explore on over the next year, perhaps starting by writing a bevy scene adaptor for some other ecosystem.

## Get Bevy Funded

Bevy is an engine capable of running just about all software *without compromise*. If you don't believe me you haven't seen my [LFConf '26 presentation](https://youtu.be/eRpMQhOR93U?si=dK8lPL4dO3cSWUWi) which demonstrated static sites, web apps, TUIs, servers, games, robots, infra deploys and agent harnesses not just running on a single engine, but **on a single data-driven binary**. Usually cross-domain capability is achieved through bridges like react-three-fiber, stitching two ecosystems together at the seam, whereas for Bevy each domain is a natural fit. 

None of this exceptional layered malleability or cross-domain capability is evident on the home page. We still present Bevy primarily as a game engine and I think this marketing should be inverted:

> before: **A game engine capable of running apps**
>
> after: **A malleable engine capable of running games**

Most software isn't as cool as indie games and the reframing is beyond the primary interests of most Bevy community members so I imagine this will be a tough sell but the counter-argument is simple: *we want the Bevy project to thrive*. Practically this means substantial funding and the value prop for a unified malleable core underpinning all of software is much bigger than that of games or even apps, and to get there we need to appeal to those beyond the games industry.

## If it ain't broke

Bevy is succeeding technically and is the healthiest online community I've ever been a part of so it would make total sense for the project to stay the course on games and GUIs. The good news is that these broader developments will continue regardless. I am having so much fun pulling on this thread of breadth and depth in software and have no intention of slowing down.

Thank you Cart, Alice and the bevy of people who created this incredible community and technological playground! 🐦
