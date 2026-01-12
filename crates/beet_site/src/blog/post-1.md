+++
title = "The Full Moon Harvest #1"
created= "2025-07-11"
+++
# The Full Moon Harvest #1 - Full Stack Bevy

I'm very excited for the `0.0.6-rc.3` of beet, now available.

<iframe src="https://www.youtube.com/embed/7koepBSRoUI" title="Full Moon Harvest #1 | Full Stack Bevy" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

<br/>
<br/>

In March I [spoke at the bevy meetup](https://youtu.be/JeXOajFv8Dk) about unifying structural representations (games, dom trees, behavior etc), since then I've only become more certain this is how I want to develop applications. One language (rust), one paradigm (bevy ecs) across the entire stack.

The biggest missing piece of this picture is the DOM so this has been my focus. I got the first implementation wrong though, thinking it would be overkill to use bevy for the web parser etc.

```js
// bevy as a feature
beet
├── html
└── bevy
```

It was working but it was not beautiful and much less maintainable, so I [threw down the gauntlet](https://discord.com/channels/691052431525675048/811674847767167027/1371290138306678795) and made the call to rewrite the entire dom parsing system in bevy.
```js
// bevy as a foundation
bevy
└── beet
    └── html
```

And that brings us to today, the first RC for the fullstack bevy experience with a growing list of features:
- File based routes
- Markdown & Rusty MDX
- All the Rendering: SSR, CSR & SSG
- Client Islands
- Server Actions
- Template Scoped Styles
- Instant Template Reloading

If this project is of interest come and say hi in the [Beetmash Discord](https://discord.gg/DcURUQCXtx), and keep me in the loop if you decide to give the quickstart a go. Its difficult to do much without stepping on a proverbial rake at the moment 😅

Happy Harvest!

Pete
