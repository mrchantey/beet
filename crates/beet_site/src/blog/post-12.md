+++
title="The Harvest #12"
created="2026-06-02"
+++

# Gentle Slopes up Lonely Mountains

*Pete Hayman — 2nd June, 2026*

<iframe src="https://www.youtube.com/embed/3V4-WM6Pc-4" alt-src="https://youtu.be/3V4-WM6Pc-4" title="The Harvest #12 - Gentle Slopes up Lonely Mountains" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

My hometown of Wollongong has a very special geography. The entire region is quite narrow because its nestled between the Pacific Ocean and one long unbroken escarpment. Once you're up there, you're up, and the boundless plateau of eastern Australia awaits.

In malleable software we talk about the [gentle slope from user to creator](https://www.inkandswitch.com/essay/malleable-software/#a-gentle-slope-from-user-to-creator), starting in userland and making our way up a (hopefully gentle) slope to some higher level of capability.

There's a bit of a catch though, often these user friendly tools trade vertical malleability for horizontal. Squarespace makes web dev accessible to users but may make game development even harder than a more generic tool, Excel helps accountants get into coding but is no good for robotics etc. This high level tooling seems to nessecitate specialization, resulting in gentle slopes that lead to lonely mountain peaks. To cross domains we need to walk all the way back down the mountain and start climbing an entirely new one, or delicately traverse narrow bridges like [react-three-fiber](https://r3f.docs.pmnd.rs/).

Game engines are a bit of an outlier here out of pure nessecity. Game dev teams are multi-diciplinary with artists, designers and engineers all working in a single environment. The games themselves are also radically diverse, the same engine is used to build a web-based card game and a multiplayer racing game, so it must fit each case very well. When a game developer travels up that path of expertise they find themselves not on a mountain peak, but a wide plateu from which they can effortlessly travel between domains of UI, 3D graphics, networking etc.

The downside is that with this wide range of capabilities many engines end up bloated. You wouldn't use Godot to write a pure html parsing cli, I'm sure its possible but it would be using a sledgehammer in place of a screwdriver. 

A notable exception is the Bevy engine, its modular `no_std` architecture makes it trivial to compile to web assembly, arduinos and everything in between, a true swiss army knife. While many niche projects can boast the same broad capabilities, Bevy is way ahead of the pack in terms of ergonomics and adoption. The binary is small, the community is large, and the architecture is beautiful.

So thats why I'm building my cross-domain malleable application framework on the Bevy engine, and its why I can comfortably work on Beet for years assured that its the right direction. In order to build a true escarpment technology its the *only* direction.

## Fun with TUIs

In other news this month I implemented the terminal ui renderer. Beet will support web and tui renderers equally so this means ensuring the style system can be adapted to css as well as terminals. This was a lot of fun, nerding out on flex containers etc. and theres a kind of purity to terminal ui, character cell rendering is like this goldilocks middleground between plain text editors and the pixel perfect dom.

I'm also very bullish on stdout as the natural fit for agents. Plawright requires a full browser pipeline and offers agents a choice between entirely unstyled accessiblity trees, and screenshots which are expensive and slow to parse. Even local models can determine if a button is centered in a tui, and when they share the same style system there is some level of guarantee the layout will be the same in the web.
