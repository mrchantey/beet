# Concepts
<!-- keep all code references in sync with docs please -->

The terminology from this library is a wip and will likely change.

## `Actions`
*aka `tasks`*

Actions are something to be performed for a given node, in beet this a combination of a `Component`, the data required for the task, and a `System`, the accompanying logic for that task. They usually either get an agent to do something or send information to their parent.

Here are some examples of actions:

- `SucceedInDuration`
	- After a given `Duration`, set `RunResult` to `RunResult::Successs`
- `Hover`
	- The entity will slowly move up and down in a sinusoidal pattern.

Notice the primitive nature of these actions, they could be combined to create a "hover for duration" node.

## `Selectors`

*aka `thinkers, parent nodes, composite nodes`*

Selectors are added to parent nodes and decide which children to run. 

Here are some examples of selectors:
- `SequenceSelector`
	- An action that runs all of its children in order until one fails.
	- Logical AND - `RUN child1 THEN child2 etc`
- `FallbackSelector`
	- An action that runs all of its children in order until one succeeds.
	- Logical OR: `RUN child1 OTHERWISE child2 etc`
- `ScoreSelector`
	- A Utility AI action that observes the current `Score` of each child and selects the highest to run.

## Common Components

- `Running`
	- Indicate this node is currently running.
- `RunResult`
	- Notify their parent that this node has finished.
- `Score`
	- Notify the parent how favourable it would be for this node to run.
- `RunTimer`
	- How long the action has been running for.

## Graph Terminology

- `BehaviorNode`
	- Literally a wrapper of a `Vec<Action>`. These can be either leaf or parent nodes.
- `BehaviorGraph`
	- A `petgraph` collection of Nodes and directed links between them. 
	- It *can* be cyclic but I prefer not to do that because it makes graphs more reusable and easier to reason about.
- `Agent`
	- Any entity that has some components, ie ie `Transform` or `MotorSpeed`, to be read or modified by the behavior graph.
