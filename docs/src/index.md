# Beet

Beet, or *bee-tee*, is an AI behavior library for games and robotics built in Rust.

> *Very early stage warning:*
> - breaking changes on patch versions
> - continued development not guaranteed
> - docs are wip
> - bugs ahoy

## Features


### 🔥 Fast
All actions are run in parallel thanks to the spectacular `bevy_ecs` scheduler.

### 🌳 Modular

Like the ECS model, each node (entity) is simply a list of actions (components). Action Graphs can be composed of other graphs, allowing for epic composability. 

> All examples in these docs are **trees**, ie non-cyclic directed graphs. This is not a techical limitation, but a choice to encourage modularity.

### 🌈 Multi-paradigm

Mix-and-match techniques from different Behavior Selection approaches for each point in the graph. I'm focusing on Behavior Trees & Utility AI at the moment because they seem to strike a nice balance between predictability and flexibility, but hope to tackle GOAP selectors at some point.

### 🌍 BYO architecture

Beet is built in Bevy, specifically the lightweight `bevy_ecs` crate, which can target epic gaming rigs and tiny microcontrollers alike. Of course if you're using Bevy (or just `bevy_ecs`) as your app framework there will be no need for blackboards etc but this is by no means a requirement. The [Beet Playground](https://github.com/mrchantey/beet/blob/main/crates/beet_web/src/bee/bee_game.rs) is a great example of a non-ecs application that uses the `beet_net` pubsub framework to sync with the AI.




All live examples use the [beet playground](https://mrchantey.github.io/beet/play?spawn_bee=1), a small web app for demos.

<iframe src="https://mrchantey.github.io/beet/play/?spawn_bee=1&spawn_flower=1&tree=CAAAAAAAAABOZXcgTm9kZQEAAAAAAAAAAAAAAAAAAD%2FNzMw9AAAAAAAAAAA"></iframe>



