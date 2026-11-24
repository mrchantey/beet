+++
title = "Building an Atmospheric OS"
description = "The endgame for the ATProto application layer is a malleable operating system"
tags = ["malleable-software", "atproto"]
created = "2026-10-21"
authors = ["Pete Hayman"]

[RouteMeta]
slug = "building-an-atmospheric-os"
+++


In this post I want to elaborate on exactly what the problem is, how Bevy and ATProto intersect meaningfully, and why combining them produces a new kind of operating system.

## Beyond kernels

> Patchwork is not an operating system, in the same way that Automerge is not a database.
> [PVH, 2026](https://www.youtube.com/watch?v=4UxGijnuXEs&t=468s)

I think the joke is that while Patchwork and Automerge do not *at all* resemble the status quo that they 'are not', they certainly replace it. It feels like now is the time to start thinking about these terms from first principles.

The first systems for operating computing machines were largely analogue, with human operators managing *both the electronics and administration*, keeping physical records for schedules and user identity. The role of the operator was holistic and involved systems oriented around users and their tasks, not hardware.

The world has changed a lot since since the punchcard batch monitors of the '50s. Instead of many people sharing one machine, each person has many machines. I haven't genuinely shared a device with another user since high school yet we still talk about an OS as being hardware bound. The system of completing computational tasks has always been broader than a single piece of hardware, and treating it so has left the data and software for deskop work entirely stranded from mobile, unless of course we kneel and kiss the ring of the Google Workspace king.. Remember those WHATWG cooporation incentives?



## Documents are the new files


Webtiles push interop a little further with the wishes api, a bit like an iframe `postMessage`, but this is still at the granularity of the widget itself rather than the individual entities and components the app is made of. Webtiles have the same *swappable not malleable* characteristics as the protocol itself, I can swap out one for another but cannot make fine-grained changes within the software itself.

- [`ing.dasl.masl::tile::resources`](https://lexicon.garden/lexicon/did:plc:izttpdp3l6vss5crelt5kcux/ing.dasl.masl/docs#IngDaslMaslMaslResources) holds urls to html pages and the resources they require for exposing through the sandbox


A computer operator managing the machine with programmers providing the domain specific tasks.


## References

- [A Discorse Disscussion](https://discourse.atmosphere.community/t/web-tiles-as-the-base-for-lexicon-embeds) of the use of [Web Tiles](https://webtil.es/) as the foundation for lexicon embeds
- [ATStore](https://atstore.fyi/), an ATProto app store
- [Operating Systems, Unbound](/assets/blog/operating-systems-unbound.html), a site i vibe coded for OS reasearch
