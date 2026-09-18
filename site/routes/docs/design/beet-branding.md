+++
title = "Beet Branding"
+++

# Beet Branding

The deliberately small single reference for the beet brand.

## Mission

To make homegrown tech ordinary, so that growing your own software in place, on any surface, in your own account, is as unremarkable as tending the garden in your back yard. The [home page](/) says what beet is, the [about page](/docs/about) argues its principles and the [glossary](/docs/glossary) fixes its words, so this page keeps only what makes an asset look and sound like beet.

## Audience

Persona: Alex

Alex is an intermediate level software engineer working at a consultancy. She is sold on tech sovereignty and has vibe coded a few ATProto apps in her spare time. When she hears malleable software she thinks OpenClaw: Powerful and flexible but unwieldy.

Alex is skeptical of new frameworks with snake-oil cross-domain claims, but pays attention when open standards are discussed because credible exit is very important to her.

Alex should have understood the three principles on the [home page](/) within her first minute.

## Stories

The people beet is built for, each as a short story of a day with it. They all matter, they are consistent with each other and no single one is primary. Some run ahead of what is built today, and the construction sign on the front page covers the gap.

- **The publication maintainer** opens their site on a phone, signs in, edits a post's prose in a markdown-shaped editor and publishes. They write a new post from the site, add a docs page and reorder the sidebar, and they can change how a post is shown, because the post and the site are the same kind of thing.
- **The git diehard** keeps a directory full of markdown and BSX and refuses the browser. One command imports the tree into a repo as scenes and blobs and the site serves from the repo. Markdown comes back where markdown went in, with a warning where honesty runs out, and beet's records stay opaque to them, which is fine.
- **The visitor who tinkers** reads someone's page, opens the editor, changes the title and keeps browsing with it. A reload keeps it, a reset button discards it and nothing reaches the author. Later a button offers to upload the change into their own repo.
- **The homegrown builder** starts from an empty repo and grows a todo list in place, a schema, a form, a table, a rule. It runs in their tab, over ssh on a friend's terminal and on a robot, from the one repo.
- **The agent**, asked to add a dark mode toggle, edits scenes through beet's own interface or through a markdown or BSX view in and out. The result is an ordinary scene edit, reviewable as a diff of the view and undoable. Files on disk are not the agent's medium either, though native tools exist for it.
- **The atproto blogger** has a PDS and no git and wants a page. One action makes them a repo, their handle resolves to it, their posts are records in their own account, and beet.org is the first host that renders it.
- **The operator on a terminal** has the same repo, no wasm and a binary lacking some capability. What it cannot run stays inert, and prose and structure are still editable from the tty.

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

## Analogies

Analogies come from lived experience and are lightly indicated, never worked to death. The Harvest names the blog and seasons the sign-offs while the content never becomes about farming, and the same restraint holds for the DJ theme, the escarpment and the festival circuit. The Bluesky and ATProto analogy, beet as one implementation of an open standard the way Bluesky is one view of ATProto, is a body text explainer used where a reader needs it and never headline material.

## Process

Headline copy, ie the hero text, taglines and page-opening sentences, is written by Pete. Agents supply the points a heading should cover and edit for consistency afterwards, they do not draft headline copy.

## Page roles

Each entrypoint has one job and its own register: 
- `site/routes/index.md` The home page is the poster, the broadest audience and the fewest words, hero plus invitation plus proof. 
- `site/routes/docs/index.md` The docs index is the orientation, where a curious builder forms the mental model. 
- `site/routes/docs/glossary.md` fixes every defined word, `site/routes/docs/about.md` argues the principles and this page holds the stories. Every other page cites them rather than restating them.
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
