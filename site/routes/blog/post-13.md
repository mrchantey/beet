+++
title="Engines, not frameworks"
created="2026-06-29"
+++

# Engines, not frameworks

One of the hardest parts of working on something from first principles is learning how to talk about it. I've been working on beet full time for three years and am still frequently updating the story.

Until now I've been using the term "Malleable Application Framework", Which in some ways fits perfectly:
- Game engines are naturally **malleable**
- The Beet entrypoint is a Bevy **App**
- Beet offers features usually associated with **frameworks**

Yet in other ways, it holds some tensions:

- The malleable software essay calls out for "tools, not apps", describing the common defnition of apps as 'avacado slicers', doing one thing well but nothing else.
- Beet is built on a game engine and game engines aren't frameworks.

## Frameworks are avacado slicers

> *A framework is ... an abstract design for solutions to a family of related problems*
> [Johnson, Ralph & Foote, Brian - Designing Reusable Classes (1988)](https://www.researchgate.net/publication/215446177_Designing_Reusable_Classes)

It makes sense that the avacado slicer factory reflects the same philosophy. The framework author had the application or something similar to it in mind when they were designing it, and created abstractions to exactly resemble that.

Abstractions are great. They're what give us maximum leverage for minimum effort. The problem is when the framework restricts our capabilities through an overly specified abstraction, resulting in tech silos. Signal-based ui frameworks aren't a great fit for games, GameObject abstractions aren't a great fit for web ui etc.

## Engines are knives

The Doom Engine was the first instance of cleanly seperating the executable from the application. `doom.exe` contains capabilities but is entirely driven by a serializable WAD file. As a result doom is still being modified and hacked at even thirty years later.

In my mind the difference here is that the problems that engines are built to solve are essentially *everything*, games are themselves mini-worlds. UI, networking, high level control flow etc might be their own bespoke framweworks in other domains, but in a game engine these are merely components sharing the same unified architecture.

## Tool Engines

Often this broad scope results in game engines having massive binaries and heavy editors. Bevy is an outlier here, simultaneously offering rendering features like Unreal nanite, *and* running on a no_std microcontroller. This uniquely positions Bevy to be the best pick for an extremely broad range of usages. That said, there's still a lot of work to be done to bring those capabilities up to industry standard expectations across all these domains, and thats where Beet comes in.

The engine model is a natural fit for `tools, not apps`, offering generalized architectural decisions not found in frameworks.



## BSX

On the technical side, this month a lot of work has been put into removing the dependency on rust for the application layer. Bevy's bsn asset format isnt ready yet, and will not be no_std at least at first, also xml markup feels much more natural for building ui, routers etc and thats what I'm primarily using it for at the moment.

That said, I wouldn't be surprised if this gets ripped out at some point in the future. DSLs are a major buzzkill, and they inevitably evolve into a crappy subset of JSX. At some point, writing a Vite plugin will probably be the way to go, but until then bsx help to keep the binary/tool divide clean.

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
