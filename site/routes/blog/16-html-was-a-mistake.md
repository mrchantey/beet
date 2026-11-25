+++
title = "HTML was a mistake"
description = "HTML was designed for documents, not software, and we've been paying the price ever since."
created = "2026-09-26"
tags = ["malleable-software", "atproto"]
authors = ["Pete Hayman"]

[RouteMeta]
slug = "html-was-a-mistake"
+++

This is part three in what has become a mini series on standardizing malleable software in the Atmosphere. In [ATProto isn't malleable yet](./atproto-isnt-malleable-yet) I described the protocol as *swappable but not yet malleable* in the sense of making arbitrary fine-grained tweaks to apps and infra. In [Bevy standardizes malleable software](./bevy-standardizes-malleable-software) I proposed the Bevy Engine and its scene format as a great fit for this, standardizing malleable software in three layers. In this post I'd like to reflect on the current state of things and how we got here.

If documents were the extent of ATProto ambitions, we could just collectively bless the Leaflet content lexicon, or a subset of HTML itself, and be done with it. But documents are far from the extent of ATProto ambitions, in fact it's harder to determine what user-facing software *should not* be Atmospheric. We need powerful application layer standards and none exist yet.

## Data, not Domains

> *We don’t know who discovered ~~water~~ **html sucks**, but we're pretty sure it wasn't a ~~fish~~ **millennial***
> *— Marshall McLuhan, 1966*

HTML is not only the world's most successful document format, but also the closest thing we have to a standard for the application layer. It's so ubiquitous that it's easy to forget the failings, the spec is now a 1500 page monolith controlled by WHATWG corporations with profit-driven incentives. It has aged poorly in the exact ways [Alan Kay predicted 30 years ago](https://youtu.be/oKg1hTOQXoY?t=1422), teasing *'this is what happens when you let physicists play with computers'*. The original HTML spec was designed purely for static documents, a domain where extensibility and malleability are naturally low priority. The round-peg-square-hole problem we have been facing in the decades since is that we want to share all kinds of things on the web, not just documents.

> *HTML on the internet has gone back to the dark ages because it presupposes that there should be a browser that should understand its formats.*
> *— Alan Kay, The Computer Revolution Hasn't Happened Yet, 1997*

I love how Kay repeats the phrase *HTML on the internet* throughout this talk, these days it's easy to forget they are not the same thing.

There's another reason HTML sucks, it is a model of inheritance, not composition:
- Node 
	- CommentNode
	- TextNode
	- ElementNode
		- Div
		- Anchor
		- ..

A node can only ever be one thing, and while inheritance was all the rage in the 80s it has aged very poorly. Interestingly, Alan Kay himself originally supported inheritance but eventually did a 180, distancing his concept of OOP from it.

The most obvious place where inheritance fails spectacularly is in video games. In even the simplest of games, inheritance turns to spaghetti very quickly. This learning is reflected in the patterns proposed by the engines themselves:
- 1998: Unreal Engine's Actor model is fully inheritance-based.
- 2005: Unity's GameObjects are much more composable, but the components themselves still encourage inheritance.
- 2022: Both engines are leaning heavily into fully composable data-oriented models like ECS, distancing themselves from their own initial architectures.

In both cases, the reception has been mixed. Game engines are incredibly complicated and it's no simple thing to port an engine's entire architecture.

Both the fixed set of browser capabilities and the inheritance model are a reflection of software attempting to model a *domain*: in the case of HTML, that of documents, with application-specific capabilities later shoehorned in via WebGL etc, which live in a different cinematic universe from the DOM.

## Hardcore Composition

Game engines like Bevy that are data-oriented from the outset have a very peculiar quality: **Absolutely nothing about their architecture is related to games**. Bevy's [core ECS crate](https://github.com/bevyengine/bevy/tree/main/crates/bevy_ecs) makes absolutely no mention of games, rendering or even geometry. It's purely about arranging data elegantly. If we are to build standards for the application layer, this is how they must work, through data, not domains.

This post is a little bit tongue in cheek. HTML is actually a fantastic standard for documents as long as it's not used for anything else, HTML is the one blessed representation of documents in beet. The difference is that it's implemented in ECS, not some domain-specific DOM, which makes it trivial for us to also have completely unrelated domains like games, servers or agentic harnesses represented equally as elegantly.
