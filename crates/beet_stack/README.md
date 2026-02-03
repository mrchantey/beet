# Beet Stack

An opinionated semantic layer between applications and interfaces like UI or humanoid robots.

The `Stack/Card` convention is based on `hypercard` which pioneered many concepts in user-modifiable software. 
Stacks represent something like a directory and cards represent something like a document (content), but also with behavior (tools).


## Supported Interfaces (wip)

- `stdio`: Event-driven command-line interface
- `ratatui`: Terminal user interfaces
- `dom`: Web-based interfaces
- `wgpu`: Bevy's native ui rendering
- `clanker`: LLM tool calls and context trees
- `embedded`: Microcontrollers like the ESP32
