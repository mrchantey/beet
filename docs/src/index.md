# Beet

Beet is a task switching library suitable for game AI, robotics & other performance-critical environments.


<iframe src="https://mrchantey.github.io/beet/play/?spawn-bee=&spawn-flower=&hide-graph=&graph=CAAAAAAAAABOZXcgTm9kZQEAAAAAAAAAAAAAAAAAAD%2FNzMw9AAAAAAAAAAA"></iframe>

*Examples are built with the [beet playground](https://mrchantey.github.io/beet/play?spawn-bee=1), feel free to experiment with the bee's graph then spawn some to see the effect.*

## Features

### 🌳 Modular

Beet follows the ECS architecture, each node (entity) is simply a list of actions (components). Action Graphs can be composed of other graphs, allowing for epic code reusability.

### 🌈 Multi-paradigm

Mix-and-match techniques from different Behavior Selection approaches for each point in the graph. Currently only Behavior Tree and Utility AI techniques are supported, but the architecture is highly extendable, allowing for state-machine transitions, GOAP, etc.

### 🌍 Flexible

Beet is built upon the lightweight `bevy_ecs` crate, which can target epic gaming rigs and tiny microcontrollers alike. Of course if you're using Bevy as your app framework there will be no need for blackboards etc but this is by no means a requirement. 

> If you would prefer to read code the [Beet Playground](https://github.com/mrchantey/beet/blob/main/crates/beet_web/src/bee/bee_game.rs) is a great example of a non-ecs application that uses the `beet_net` pubsub framework to sync with the AI.