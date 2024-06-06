# Reinforcement Learning

The Bevy/Rust ecosystem is uniquely positioned to be a fantastic choice for RL.
- Bevy is the worlds fastest game engine with its level of capabilities.
- Candle allows for 100% Rust RL implementations.


## Training Cycle


This example uses the frozen lake

1. `QTableSelector` runs an action:
	- Setting the `GridDirection` action on the agent
	- Setting the child action to `Running`, which activates `TranslateGrid`
3. `TranslateGrid` runs until completion, at which point it updates the `GridPos` state
4. `reward_grid` runs directly after, updating the reward for the action taken
5. `QTableSelector` evaluates the reward and selects the next action.
