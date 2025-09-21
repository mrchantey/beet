> This document is for testing out pdf print formatting.
> print-to-pdf `ctrl+p` to see hidden elements, page breaks etc.

# Beet: A tale of three blog posts

*ai slop at its finest* 🙄

## Chapter I – Awakening

In the twilight of code and wire, before Beet came to be, the lands of Web and Game sat apart—each governed by its own laws, its own languages, its own paradigms. Developers built walls of abstraction, bridges of glue code, towers of configuration. The Bevy engine reigned in the realm of games: fleshed with ECS, systems, components, coordinators. But the Web’s kingdom, with DOM trees, routers, rendering strategies, beckoned for unity.

Then came Beet. A whisper at first, from one who believed in “one language (Rust), one paradigm (Bevy ECS) across the entire stack.”

 Beet would span Web and Game, DOM as structural first‐class, router and server actions as systems, SSR/CSR/SSG together in harmony.

 From the forge of its creator, the first release candidate emerged: file based routes, markdown & MDX, instant template reloading, template scoped styles.

 And the full‐stack vision dawned.

<div class="bt-u-page-break"></div>


## Chapter II – The Long Harvest

In the green fields beyond the hill of legacy frameworks, the Long Harvest began. Beet’s seed was sown in the fertile soil of Bevy’s Entity Component System. The first tendrils of its roots reached into the Earth of DOM trees: the mighty structure of HTML and of behavior, once separate, now intertwined under the banner of ECS. The heavens themselves seemed to shift: here was a paradigm that could render (SSR), hydrate (CSR), generate (SSG), all by the same soul.


As seasons passed, Beet sprouted new branches. The router became more than just paths and handlers—it was Bevy‐style: routes as bundles, children, filters. Middleware was no longer foreign code to be patched in, but systems in the grand orchestra of requests and responses.

 Server Actions—once a separate concern—became regular Bevy systems, receiving inputs, processing, outputting results. No longer did the developer dance between disparate models; everything spoke Bevy.


And there were islands—Client Islands, carved into the Web, scenes embedded within HTML, each one alive with Bevy’s ECS, yet gentle with complexity. No codegen labyrinths, no macros binding the soul. These islands responded, updated, connected.

 Static files, once burdens handled poorly by servers or overloaded functions, found their place in buckets—S3 or similar—served alongside the generated HTML. The harvest grew rich.


Yet the journey was not without trial. The old habits of abstraction, macros, discriminated unions lurked in shadows. The creator—Pete—wrestled with config files deemed burdensome; the CLI tasks regarded as fragile. He saw complexity grow, maybe out of fear, maybe out of trying too much too soon. So he cut away: simplifying config, making declarative route definitions, letting Bevy’s simple principles guide architecture.


Then the night sky cracked with revelation. Behavior trees, DOM trees, even AI hierarchies—they all yielded to Bevy’s dominion: entities and components. Traits and macros and enums were tools, not masters. The world of Beet came alive. In the forge of “Simple Is Hard,” its creator learned that the elegant path is often the hardest to walk.


And in that Long Harvest, Beet matured: with DOM diffing beginnings (a TODO app supporting insertion and removal of DOM nodes), with community growing warm, ideas exchanged late into the night. The Developer’s heart swelled. For the first time, Web and Game were not strangers—they shared blood, bones, structure. Beet was both seed and scaffold. The harvest promised a new age.


<div class="bt-u-page-break"></div>

## Chapter III – The Dawn of Bevy’s Five

Now, the dawn breaks. Bevy’s Five—five pillars of change—rise: ECS everywhere, simplicity as discipline, unified rendering, declarative structures, and community forged in code. The way of Beet is alive.

 The creator, having wrestled with abstractions and rewrites, stands beneath the full moon (Harvest #3), reflecting on the deviations, the false turns, the patterns discovered and discarded.


Beet now breathes. Behavior as entity hierarchies, DOM diffing in motion, routing fearless, systems everywhere. The full‐stack Bevy experience: web parser, SSR, client islands, server actions, style scoping—all harmonized. The community gathers. The vision stands. A new age begins.
