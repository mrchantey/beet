+++
title = "beet_ml"
+++

# beet_ml

`beet_ml` brings machine learning into the action model. Like [beet_spatial](/docs/crates/beet_spatial) it builds on [beet_action](/docs/crates/beet_action), so a model invocation is an action that slots into a behavior tree alongside everything else.

It currently spans two domains:

- **Language**: the `Bert` sentence-embedding asset, running on [burn](https://burn.dev) with selectable wgpu, ndarray or cuda backends, paired with a `Sentence` action that picks the closest match to a user phrase. This is enough to route natural language to behavior without a cloud round trip.
- **Reinforcement learning**: a `FrozenLake` environment and Q-learning agents ported from OpenAI Gym, runnable headless for training or in realtime to watch them learn.

Add `BeetMlPlugins` to register the assets, actions and tick schedule.
