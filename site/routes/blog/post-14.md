+++
title="ATProto isn't malleable yet"
created="2026-08-18"
+++

# ATProto Isn't Malleable Yet

I'm a malleable software nerd, I love thinking about applications and how we can improve them from first principles. DWeb Camp '26 was my first opportunity to talk to people who do the same at the protocol layer. It was also the first time I heard Brewster Kahle eccentrically yell on stage that we are "Locking the web open!" and in that moment something clicked for me, the application layer is only half the battle, its the *whole damn system man!*

Later that day I was introduced to Kevin Triplett who explained that while the [Project Weave](https://projectweave.tech/) protocol standards are reaching maturity, the picture is incomplete without malleable application standards. It feels like we're builing two sides of the same coin.

## ATProto is swappable

In this post I am using ATProto as an example but a similar reasoning can be applied to other open protocols like Matrix or ActivityPub. The ATProto data storage, feed generators and client apps can all be seamlessly swapped out for another, but this is not to say that the components themselves are easily modified. Whether I am using BlueSky, BlackSky or any other service, the provided feed generators and client apps are black boxes exposing a handful of knobs determined by product designers with specific use-cases in mind. Adding a button or filter means forking, modifying and deploying an entire new instance of that component.

In March BlueSky [announced Attie](https://mashable.com/article/bluesky-announces-attie-ai-app-for-custom-feeds) and BlackSky [announced Acorn](https://blackskyweb.xyz/introducing-acorn-community-infrastructure-that-grows-with-you/), both offering customizability not available in the vanilla clients and algorithms. While on the surface these appear to be malleable solutions, code is a brittle instrument even when AI generated, as outlined in the [malleable software essay](https://www.inkandswitch.com/essay/malleable-software/#ai-assisted-coding). To me the deepest issues with code-first approaches are composability and security, each program is itself a silo and a challenge to sandbox. We need more powerful abstractions.

## Data, not code

After DWeb Camp I [presented Beet at Local-First Conf '26](https://youtu.be/eRpMQhOR93U?si=-elrJ5NFsi2FZ0Bw), demonstrating how a *single binary* can be used to run a game, a web app, an infra deploy, an AI agent and a robot, all without scripting. This can only be done with data-driven software, the code simply offers a collection of systems ready to operate on the serialized data it is provided.

The most powerful form of data-driven software I have encounted is the scene graph used by Entity Component System frameworks. ECS is conceptually a very simple architecture, and so is its serialized representation.

```json
{
	"0": { 
		"org.beet.Button": {},
		"org.beet.RunScript": "console.log(\"hello world!\")" 
	},
	"1": { 
		"org.bevy.ChildOf": "0",
		"org.bevy.Text": "Click Me!"
	}
}
```

Scripting is still supported, but now sandboxed via QuickJS with fine-grained control over capabilities as opposed to coarser techniques like iframes and web workers. It also has the added benefit of being highly portable, using the same runtime in native, browser and even microcontroller environments.

## Locked open applications

The good news is that nothing needs to change at the protocol layer to add malleability. We can already build clients and feed generators capable of interpreting a **scene lexicon**, and running user provided scenes in a safe and composable manner.

Malleable ATProto components are just the beginning. With this fine grained control we can write clients that interpret posts defined as scenes, sharing widgets and games just as we share text and video. And, as Thomas Kelly proposes, for even more powerful customization users could share pluggable wasm modules to execute some or all of that behavior!

Its early days but perhaps once these ideas are proven out we could discuss a standardized format for locked open malleable software, I think interoperability would be a great benefit for the ecosystem.

This is the area that I am building in, if it sounds of interest check out [beet](https://beet.org) or come and say hello in the [Beet Roomy space](https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm).