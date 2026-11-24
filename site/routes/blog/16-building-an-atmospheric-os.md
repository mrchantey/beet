+++
title = "Building an Atmospheric OS"
description = "The ATProto endgame is a malleable operating system"
+++

A computer operator managing the machine with programmers providing the domain specific tasks.

This is part three in what has become a mini series on standardizing malleable software in the Atmosphere. In [ATProto isn't malleable yet](./atproto-isnt-malleable-yet) I described the protocol as *swappable but not yet malleable* in the sense of making arbitrary fine-grained tweaks to apps and infra. In [Bevy standardizes malleable software](./bevy-standardizes-malleable-software) I proposed the Bevy Engine and its scene format as a great fit for this, standardizing malleable software in three layers. In this post I want to elaborate on exactly what the problem is, how Bevy and ATProto intersect meaningfully, and why combining them produces a new kind of operating system.

Webtiles push interop a little further with the wishes api, a bit like an iframe `postMessage`, but this is still at the granularity of the widget itself rather than the individual entities and components the app is made of. Webtiles have the same *swappable not malleable* characteristics as the protocol itself, I can swap out one for another but cannot make fine-grained changes within the software itself.

- [`ing.dasl.masl::tile::resources`](https://lexicon.garden/lexicon/did:plc:izttpdp3l6vss5crelt5kcux/ing.dasl.masl/docs#IngDaslMaslMaslResources) holds urls to html pages and the resources they require for exposing through the sandbox


## How Standards Proliferate

> *We don’t know who discovered **html sucks**, but we're pretty sure it wasn't a **millenial***
> *— Marshall McLuhan, 1966*

HTML is the worlds most successful document format, so ubiquitous its easy to forget the failings; The spec is now a 1500 page monolith controlled by WHATWG corporations with profit driven incentives. It has aged poorly in the exact ways [Alan Kay predicted 30 years ago](https://youtu.be/oKg1hTOQXoY?t=1422), teasing *'this is what happens when you let physicists play with computers'*. The original HTML spec was designed purely for static documents, a domain where extensibility and malleability are naturally low priority. The round-peg-square-hole problem we have been facing over the decades since then is that we want to share a lot more on the web than documents and shoehorning is painful.

> *HTML on the internet has gone back to the dark ages because it presupposes that there should be a browser that should understand its formats.*
> *— Alan Kay, The Computer Revolution Hasn't Happened Yet, 1997*

I love how Kay repeats the phrase *HTML on the internet* throghout this talk, these days its easy to forget they are not the same thing. If documents were the extent of ATProto ambitions we could just collectively bless the Leaflet content lexicon or a subset of html itself and be done with it. But documents are far from the extent of ATProto ambitions, in fact its harder to dermine what user facing software *should not* be Atmospheric.


## Beyond kernels

> Patchwork is not an operating system, in the same way that Automerge is not a database.
> [PVH, 2026](https://www.youtube.com/watch?v=4UxGijnuXEs&t=468s)

I think the joke is that while Patchwork and Automerge do not *at all* resemble the status quo that they 'are not', they certainly replace it. It feels like now is the time to start thinking about these terms from first principles.

The first systems for operating computing machines were largely analogue, with human operators managing *both the electronics and administration*, keeping physical records for schedules and user identity. The role of the operator was holistic and involved systems oriented around users and their tasks, not hardware.

The world has changed a lot since since the punchcard batch monitors of the '50s. Instead of many people sharing one machine, each person has many machines. I haven't genuinely shared a device with another user since high school yet we still talk about an OS as being hardware bound. The system of completing computational tasks has always been broader than a single piece of hardware, and treating it so has left the data and software for deskop work entirely stranded from mobile, unless of course we kneel and kiss the ring of the Google Workspace king.. Remember those WHATWG cooporation incentives?


## Documents are the new files


## References

- [A Discorse Disscussion](https://discourse.atmosphere.community/t/web-tiles-as-the-base-for-lexicon-embeds) of the use of [Web Tiles](https://webtil.es/) as the foundation for lexicon embeds
- [ATStore](https://atstore.fyi/), an ATProto app store
- [Operating Systems, Unbound](/assets/blog/operating-systems-unbound.html), a site i vibe coded for OS reasearch
