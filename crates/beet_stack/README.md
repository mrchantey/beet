# Beet Stack

An opinionated, interface-agnostic application representation, inspired by `hypercard` which pioneered many concepts in user-modifiable software. 

### Stack

A stack is something like a directory, containing one or more cards.

### Card

A card is something like a file, a single unit of representation in an interface, these are usually only represented one at a time. 

Cards may contain Content, Tools, and nested Stacks.

### Content

Static or dynamic information to be presented to the user, like text or images.

### Tools

Tools are affordances surfaced in an interface, every tool has an input and output type.

### Interfaces (wip)

Interfaces are ways of representing a card, presenting its content and tools for interaction.

- `stdio`: Event-driven command-line interface
- `ratatui`: Terminal user interfaces
- `http`: A http server interface
- `dom`: Web-based interfaces
- `wgpu`: Bevy's native ui rendering
- `clanker`: LLM tool calls and context trees
- `embedded`: Microcontrollers like the ESP32
