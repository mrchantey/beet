+++
title="The Harvest #10"
created="2026-04-06"
+++

# Application Level Homoiconicity

As much as I'd like to be I'm not a lisp guy, when somebody tells me `(+ 1 2)` is so cool because its all just data, I believe them and can admire the symmetry but have not yet built up an intuition for why thats useful.

I'm a games guy, I've seen how messy hard-coded control flow gets in patterns like enemy AI. I've also seen how brittle it is to shoehorn in (and often back out) some highly opinionated DSL to address this.

The core idea of `beet 0.0.1` was an application level homoiconicity, piggybacking the exact same scene hierarchies we're already using for scene layouts and UI to describe control flow patterns like behavior trees.

```rust
fn enemy_ai() -> impl Bundle {
	(
		Fallback,
		children![
			Attack,
			Patrol,
		]
	)
}
```

In bevy this translates to **Behavior Expressed as Entity Trees**, a backronym I thought of soon after releasing beet. Technically they aren't limited to just trees, although they are strongly preferred over looser patterns like state machines for the same reasons that GOTO is considered an antipattern.

Its a simple idea, the hard part is doing it well. Upon release, beet was actually the third iteration of this idea, and while the public api for this pattern has remained the same in the two years since its release, the implementation has evolved dramatically. But with every rewrite it gets closer, feels more native and with the grain of the existing bevy ecosystem.

In the last month I've fully rewritten the implementation for what feels very close to the ideal form. `beet_flow` will be deprecated in favour of `beet_tool`, instead of juggling multiple observers and a dangerous `Arc<Mutex<Option<T>>>::take` pattern for non-clone types, the api is now just an async function.

```rust
/// run all children in sequence,
/// propagating the input or short-circuting
/// with the 'output' if one fails.
async fn sequence::<In,Out>(mut input: In, entity: AsyncEntity) -> Outcome<In,Out> {
	for child in entity.children().await {
		match child.call.await {
			Outcome::Pass(next_input) => {
				*input = next_input;
			}
			Outcome::Fail(fail) => {
				return Outcome::Fail(fail)
			}
		}
	}
	Outcome::Pass(input)
}
```

## Example: Router

For many behavior tree usages both the input and output are just a unit type `()`, but the propagation pattern is extremely useful for patterns like routers, where the `Request` and `Response` types are not clone, and we want to fall back to a 404 handler.

Here's the router for the server of this web page. The Request is propagated without clone until a match is found.

```rust
fn default_router() -> impl Bundle {
	(
		Fallback::<Request, Response>::default(),
		children![
			// short circuit on `help` query param
			HelpHandler,
			// try to find a matching route
			RouteHandler,
			// finally fall back to not found
			NotFoundHandler,
		]
	)
}
```

## Example: Agent Harness

These primitives are very versatile, here's a simple agent chat loop.

```rust

fn chat_agent() -> impl Bundle{
	(Repeat::new(), children![(
		Thread::default(),
		Sequence::new()
			// skip static actors like the system
			.allow_no_tool(),
		children![
			(
				Actor::new("BeepBot", ActorKind::System), 
				children![
					Post::spawn("you are robot, make beep boop noises")
				]),
			(
				Actor::new("BeepBot", ActorKind::Agent),
				// tool calls are just nested actions
				OpenAiProvider::gpt_5_mini().unwrap(),
				children![add_tool()]
			),
			(
				Actor::new("User", ActorKind::User), 
				// this action just gets user input
				StdinPost
			),
		]
	)])
}
```

And there you have it, serializable homoiconicity for common application logic.

## Serialization is key to malleability

The gains in simplicity are self-evident but i think thats just the beginning:
- What productivity boosts can we get by writing application logic in a game engine editor?
- How can infra deployments be simplified when the router logic itself is just a json file?
- What capabilities does an agent get when it can confidently modify its own behavior without source code changes?

ECS is a weird architecture in that its so agnostic to the application, and the scope of beet has evolved beyound a simple behavior library into a full stack framework to explore this. It feels like we're on the ground floor of discovering the implications of this architecture, and I can't wait to see how the landscape unfolds.