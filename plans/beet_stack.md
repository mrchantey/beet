# Beet Stack

Interface agnostic

## Content

Types with semantic markup, with optional rendering params.

## Tools

Way to interact with a program, using a generalized `Request / Response` pattern.

## Semantics

Unopinionated types for describing content:
- `Card`
- `Stack`
- `Link`
- `Paragraph`

## Layout

Uses `bevy_ui` as backend for rendered interfaces: bevy_render, ratatui, dom.

## Interfaces

### `stdio`

stdio works operates on events instead of retained types.

- `Query<Text,Added<Text>>`
	- Render created items in depth-first order
- `Query<Link,Added<Link>>`
	- actually print the url
- `Query<Text,Changed<Text>>`
	- Render diff immediatly, tracking previous changes in a `Local`
	- multiple changed Text will break this, thats ok
- `Query<Button,Removed<Disabled>>`
	- stdin for each field in the form

### `clanker`

### `ratatui`

### `dom`
