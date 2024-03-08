# Concepts
<!-- keep all code references in sync with docs please -->

Every behavior tree library uses its own terminology, and this one is no different 🙃

## `Actions`
*aka `tasks`*

Actions are something to be performed for a given node, in beet this a combination of a `Component`, the data required for the task, and a `System`, the accompanying logic for that task. They usually either get an agent to do something or send information to their parent.

Here are some examples of actions:

- `SucceedInDuration`
	- After a given duration, will set `RunResult` to `RunResult::Successs`
- `Hover`
	- The entity will slowly move up and down in a sinusoidal pattern.

Notice the primitive nature of these actions, they can be combined to create a "hover for duration" node. We separate the animation from the success criteria for optimal reusablility.


## `Selectors`

*aka `thinkers, parent nodes`*

Selectors are use by parent nodes to decide which children to run. 

Here are some examples of selectors:
- `SequenceSelector`
	- An action that runs all of its children in order until one fails.
	- Logical AND - `RUN child1 THEN child2 etc`
- `FallbackSelector`
	- An action that runs all of its children in order until one succeeds.
	- Logical OR: `RUN child1 OTHERWISE child2 etc`
- `UtilitySelector`
	- An action that observes the [`Score`] of each child and selects the highest to run.


## Common Components

- `Running`
	- Indicate this node is currently running.
- `RunResult`
	- Notify their parent that this node has finished.
- `Score`
	- Notify the parent how favourable it would be for this node to run.
- `RunTimer`
	- How long the action has been running for



## Graph Terminology

- `BehaviorNode`
	- Literally a wrapper of a `Vec<Action>`
- `BehaviorGraph`
	- A `petgraph` collection of Nodes and directed links between them. This *can* be cyclic but I prefer not to do that.
	- This is the "blueprint" of an agents actual graph, and its `spawn` method is used to be attached to an agent.
- `Agent`
	- Any entity that has some components to be modified by the behavior graph, ie `Transform` or `MotorSpeed`.

## Agent Lifecycle

1. Create a graph and set the initial values for all of the accompanying actions.
2. When creating an agent, call `graph.spawn(target)` which will do the following:
	- Create an entity for each node in the graph
	<!-- - Attach an `EntityGraph` to the target. *this will soon change* -->
3. When the agent is destroyed the accompanying graph is also destroyed.