# Beet Stack

Interface agnostic content and tooling conventions.

## Overview

Beet Stack provides unopinionated primitives for describing interactive content that can be interacted with across different interfaces.

## Features

- **Content**: Semantic markup types with optional rendering parameters
- **Tools**: Generalized `Request/Response` pattern for program interaction
- **Semantics**: Unopinionated types like `Card`, `Stack`, `Link`, `Paragraph`
- **Layout**: Uses `bevy_ui` as backend for rendered interfaces

## Supported Interfaces

- `stdio`: Event-driven command-line interface
- `ratatui`: Terminal user interfaces
- `dom`: Web-based interfaces
- `wgpu`: Bevy's native ui rendering
- `clanker`: LLM tool calls and context trees
