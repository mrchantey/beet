
# Unreleased

## Features

### Global Observers

We're going global! Using global action observers provides several performance and usability improvements:
- 5x faster action spawning/despawning
- OnRun 

### Automatic Action Adding

Both as a usability improvement and to facilitate global observer spawning.
`#[derive(Action)]` now implements `Component` to facilitate automatic observer adding. This means that `#[systems]` and `#[global_observers]` attributes are deprecated as we no longer have access to `App`, please add them like you would in vanilla bevy.

## Changed
- `ParallelFlow` now awaits all child results before returning, if any fail it will fail immediately.


## Removed
- `ActionPlugin`: observers now automatically added
- `TargetEntity`: Use `OnRun::target`
- `RootIsTargetEntity`: Use `OnRun::target`
- `DynamicRootIsTargetEntity`: Use `OnRun::target`