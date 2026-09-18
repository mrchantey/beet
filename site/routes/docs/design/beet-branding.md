+++
title = "Beet Branding"
+++

# Beet Branding

The deliberately small single reference for the beet brand.

## Mission

Beet is an Atmospheric OS for homegrown tech, an operating system built around a person rather than a machine. Software is a repo of scenes, records and blobs that belongs to an account, runs on every surface the account touches and is grown in place by the person who uses it. Beet is malleable, agnostic and decentralized, and the atproto integration is what those three principles were always pointing at, which is what Atmospheric means. The [principles](/docs/principles) page argues each, the [glossary](/docs/glossary) fixes the words.

## Audience

Persona: Alex

Alex is an intermediate level software engineer working at a consultancy. She is sold on tech sovereignty and has vibe coded a few ATProto apps in her spare time. When she hears malleable software she thinks OpenClaw: Powerful and flexible but unwieldy.

Alex is skeptical of new frameworks with snake-oil cross-domain claims, but pays attention when open standards are discussed because credible exit is very important to her.

Alex should have understood the three principles upon viewing the home page:
1. Malleable:
	- ATProto being swappable doesn't make it malleable, adding a button means forking and redeploying a whole client.
	- Code-first solutions are a sledgehammer, OpenClaw is brittle because it lacks fine-grained malleability.
	- Scenes, scripts and plugins are a gentle slope, and each stays open while the software runs.
2. Agnostic:
	- One repo runs in a tab, a terminal, over ssh and on a robot, and serves sites, games and deploys alike.
	- Every layer of the stack is swappable, so nothing beet stands on is a lock-in.
3. Decentralized:
	- Different clients operate on the same data (ATProto), and the software is data in the same account.
	- The scene format is separate from the beet runtime, so credible exit reaches the software as well as the data.

## Brand Archetype

The primary archetype is Creator and the secondary is Everyman. Peer brands are Raspberry Pi, Scratch and Arduino, not Vercel and Deno. The test for any asset is whether it would look at home next to a Raspberry Pi zine.

## Voice

The voice is playful, folk, warm, humble, peer and movement, all six sliders at the warm end. The risk of an all-warm voice making a standards claim is reading unserious, and the mitigation is precision underneath the warmth, never dialing the warmth back.

### Sentences

- Single cohesive sentences that flow. No colon-spliced "X: Y" constructions, no em dashes, no fragment stacks. A sentence that needs a colon to hold together is rewritten as one thought.
- Contractions and first person are fine. The blog is "I", the site is community "we", never corporate we.
- Hedge honestly on uncertain claims ("I think", "so far", "still early days") and be plain about direction. Never hype. Superlatives are banned.

### Article Structure

- Open with something concrete before anything abstract, ideally a lived moment that lands on the technical point within a paragraph.
- Short sections with punchy headers, which may riff on songs, quotes or sayings.
- Ground every abstract claim in a concrete example, ideally runnable. Code blocks and diagrams carry the argument, prose connects them.
- Link generously to sources, talks and prior posts, and quote people by name.
- End with an invitation or a forward look, not a summary.

### Docs and tutorials

- Same voice, calmer register. Second person, present tense, one idea per paragraph.
- No marketing claims inside docs, the reader is already here. State what a thing does and show it doing it.
- Humor is welcome at the edges and never in load-bearing explanations.

## Words

- Owned: Atmospheric, homegrown, repo, scene, record, blob, runtime, action, malleable, agnostic, decentralized, swappable stack, standard, interoperable, sovereign, locked open, harvest, ecosystem.
- A runtime is a binary that runs repos. The thing a person creates and shares with beet is a scene, a script, a plugin or a runtime, and never an app or a tool: the [glossary](/docs/glossary) is the source of truth for every defined word and its retired predecessors.
- Malleable and sovereign are qualities beet has, never the noun it is. "Malleable engine" is retired as a self-description.
- Reserved for beetmash: mash, remix, DJ and breaking red-tape language.
- Banned: app, platform and framework as self-description, revolutionize, blazingly, superlatives and hype generally.

## Analogies

Analogies come from lived experience and are lightly indicated, never worked to death. The Harvest names the blog and seasons the sign-offs while the content never becomes about farming, and the same restraint holds for the DJ theme, the escarpment and the festival circuit. The Bluesky and ATProto analogy, beet as one implementation of an open standard the way Bluesky is one view of ATProto, is a body text explainer used where a reader needs it and never headline material.

## Process

Headline copy, ie the hero text, taglines and page-opening sentences, is written by Pete. Agents supply the points a heading should cover and edit for consistency afterwards, they do not draft headline copy.

## Page roles

Each entrypoint has one job and its own register: 
- `site/routes/index.md` The home page is the poster, the broadest audience and the fewest words, hero plus invitation plus proof. 
- `site/routes/docs/index.md` The docs index is the orientation, where a curious builder forms the mental model. 
- `site/routes/docs/principles.md`, `stories.md` and `glossary.md` are the source of truth for the framing: every other page cites them rather than restating them.
- `site/routes/docs/scenes.md`, `scripts.md` and `plugins.md` explain one layer of malleability each and pair with the tutorial of the same layer.
- `site/routes/docs/about.md` The about page is the story, lineage, name and the beetmash relationship. 
- `README.md` The README is the workshop door, developer-facing install and run with a link back to the site for the vision.

## Palette

Light mode is print, the cream and ink of a zine handout, and dark mode is terminal phosphor, neon on a green-black screen. The pairing maps the folk and protocol registers of the brand onto beet's own render targets, the printed page and the terminal.

| Key | Hex | Notes |
| --- | --- | --- |
| Primary | `#006c4f` | Brand green, primary in both modes |
| Secondary | `#f028a8` | Hot pink, hue 346°, so light tone 40 lands raspberry `#b5007c` rather than purple |
| Tertiary | `#ea8a0c` | Harvest amber |
| Neutral, light | `#f4eede` | Handout cream, seeds light surfaces near tone 94 |
| Neutral, dark | `#14211d` | Handout ink, hue 173°, seeds green-cast dark surfaces near tone 8 (`#0d1a16`) |
| Neutral variant | `#6a7772` | Handout muted |
| Error | `#de3730` | Hue 35°, kept well clear of secondary pink |

The neutral key is split per mode, each scheme builds its surface roles from its own neutral ramp, and the inverse-surface roles cross over to the other mode's ramp, so an inverse surface in light mode is phosphor and in dark mode is print.

## Typography

Inter everywhere, with system fallbacks and mono reserved for code. Structural weights run heavy, 700 through the headlines and titles and 900 across the display scale, and every structural step is tracked at -0.03em so heavy type sets as one mass rather than as spaced letters. The wordmark is weight 900 in brand green, worn at display scale on a poster and at title scale in the app bar, while eyebrow labels are small, bold, wide tracked and authored in upper case. The engine carries all of this as tokens, the `Wordmark` and `Eyebrow` composites behind the `text-wordmark` and `text-eyebrow` classes.

## Marks

The mark is a beetroot, a secondary pink root under primary green leaves with ink root hairs, flat and two colour so it survives a 16 pixel favicon. It lives at `site/assets/branding/logo.svg` and every icon beside it is rendered from that one file, as is the 1200x630 social card whose own source sits in the same directory. Both are hand authored, never generated by an image model, which garbles text and layout.
