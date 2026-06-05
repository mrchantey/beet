# beet_ml

Machine learning actions built on [`beet_action`].

- **Language**: the [`Bert`] sentence-embedding asset (via Huggingface [Candle](https://github.com/huggingface/candle)) with a [`Sentence`] action for selecting the closest match to a user phrase.
- **Reinforcement learning**: a `FrozenLake` environment and Q-learning agents ported from OpenAI Gym, runnable headless or in realtime.

Add `BeetMlPlugins` to register the assets, actions and tick schedule.
