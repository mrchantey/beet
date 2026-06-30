+++
title="Engines, not frameworks"
created="2026-06-30"
+++

# Engines, not frameworks

*Pete Hayman — 30th June, 2026*

Until now I've been using the term "Malleable Application Framework" to describe beet, which in some ways fits perfectly:
- The ECS and scene architecture make it very **malleable**
- The entrypoint is literally a Bevy **App**
- Beet offers features usually associated with **frameworks**

Yet in other ways, it holds some tensions:

- The malleable software essay calls out for "tools, not apps", describing the common defnition of apps as 'avacado slicers', doing one thing well but nothing else.
- Framework implies a single paradigm, whereas ECS feels like a meta-paradigm; signals, game objects and other patterns can be constructed within it.

## Frameworks are avacado slicers

In construction, a framework provides the predetermined structure of the building. The building has been designed and partially built with a specific purpose in mind. [If apps are avocado slicers](https://www.inkandswitch.com/essay/malleable-software/#apps-are-avocado-slicers), it makes sense that the frameworks used to build them share the same philosophy.

Abstractions are great. They're what give us maximum leverage for minimum effort. The problem occurs when the framework restricts our capabilities through an overly specified abstraction, resulting in tech silos.

## Engines are knives

![Portable Steam Engine](https://imagedelivery.net/znDGKaxxskgDbNUpqPVRFg/ph-collection-media/images/144244.jpg/w=1080,q=75,metadata=keep)

Engines offer capability without a prescribed usage. The 19th century equivelent of a generator was the portable steam engine, simply providing a rotating flywheel to be attached to anything you choose.

The Doom Engine was the first instance of cleanly seperating the executable from the application. `doom.exe` contains capabilities but is entirely driven by a serializable WAD file. As a result doom is still being modified and hacked at even thirty years later. The executable provides capabilities, but is entirely driven by serializable scene files.

## Tool Engines

A game engine is like a collection of mini engines working together: rendering, UI, physics, networking. Often this broad scope results in game engines having massive binaries and heavy editors. Bevy is an outlier here, simultaneously offering rendering features like Unreal nanite, *and* running on a no_std microcontroller. This uniquely positions Bevy to be the best pick for an extremely broad range of usages. That said, there's still a lot of work to be done to bring those capabilities up to industry standard expectations across all these domains, and thats where Beet comes in.

The engine model is a natural fit for `tools, not apps`, providing generalized architectural decisions not found in frameworks.

## BSX

On the technical side, this month a lot of work has been put into removing the dependency on rust for the application layer. Bevy's bsn asset format isnt ready yet, and will not be no_std at least at first, also xml markup feels much more natural for building ui, routers etc and thats what I'm primarily using it for at the moment.

That said, I wouldn't be surprised if this gets ripped out at some point in the future. DSLs are a major buzzkill, and they inevitably evolve into a lesser subset of JSX. At some point, writing a Vite plugin will probably be the way to go, but until then bsx help to keep the binary/tool divide clean.

as you can see, the inheritence model that html/xml is built on isnt a perfect fit for ECS, we end up heavily leaning into this spread `<Comp1 {(Comp2,Comp3)}/>` syntax, but at least its somewhat familiar.

```jsx
<Name("Malenia") {(Health(100.0), HealingPotions(2), Repeat, RunOnLoad)}>
	<Fallback>
		<TryHealSelf/>
		<HighestScore>
			<Name("Waterfoul Dance") {(AttackPlayer{max_damage:15.0,max_recoil:30.0}, RandomScore)}/>
			<Name("Scarlet Aeonia") {(AttackPlayer{max_damage:10000.0,max_recoil:10.0}, Score(0.05))}/>
		</HighestScore>
	</Fallback>
	<Name("Elden Lord") {(Health(100.0), AttackTarget)}/>
</Name>

```
