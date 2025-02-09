
# Unreleased

## Features

### Global Observers

The `beet_flow` `OnRun` machinery now uses global observers, bringing many performance useability and usability improvements:
- 5x faster action spawning/despawning
- OnRun c 

### Automatic Action Adding

Both as a usability improvement and to facilitate global observer spawning.
`#[derive(Action)]` now implements `Component` to facilitate automatic observer adding. This means that `#[systems]` and `#[global_observers]` attributes are deprecated as we no longer have access to `App`, please add them like you would in vanilla bevy.

## Removed
- `ActionPlugin`: observers now automatically added
- `TargetEntity`: Use `OnRun::target`
- `RootIsTargetEntity`: Use `OnRun::target`
- `DynamicRootIsTargetEntity`: Use `OnRun::target`