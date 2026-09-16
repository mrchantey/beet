+++
title = "Diagrams"
+++

# Diagrams

A `mermaid` code fence renders as a diagram on every sink, painted by the material tokens so it follows the color scheme: a picture on the web, box-drawing text or a kitty raster on the terminal.

## Flowchart

```mermaid
flowchart LR
    subgraph parse [Parse]
        A[Markdown] --> B[Entity tree]
    end
    B --> C{Sink?}
    C -->|web| D[Inline svg]
    C -->|terminal| E[Box art]
```

## Sequence

```mermaid
sequenceDiagram
    participant B as Browser
    participant S as Server
    B->>S: GET /docs/design/diagrams
    activate S
    Note right of S: parse, style, render
    S-->>B: text/html
    deactivate S
```

## Class

```mermaid
classDiagram
    class MermaidDiagram {
        +String source
        +DiagramKind kind
    }
    class DiagramForm {
        Svg
        Text
        Raster
    }
    MermaidDiagram --> DiagramForm : builds
```

## State

```mermaid
stateDiagram-v2
    [*] --> Collected
    Collected --> Svg : web
    Collected --> Text : terminal
    Text --> Raster : graphics
    Raster --> Text : lost graphics
```

## Entity relationship

```mermaid
erDiagram
    ROUTER ||--o{ ROUTE : serves
    ROUTE ||--|| PAGE : renders
    PAGE }o--|| STORE : reads
```

## Pie

```mermaid
pie title Where a page's bytes go
    "Markup" : 55
    "Stylesheet" : 30
    "Script" : 15
```

## Gantt

```mermaid
gantt
    title A release
    dateFormat YYYY-MM-DD
    section Build
    Compile      : 2026-09-01, 3d
    Test         : 2026-09-04, 2d
    section Ship
    Deploy       : 2026-09-06, 1d
    Announce     : 2026-09-07, 1d
```

## Git graph

```mermaid
gitGraph
    commit
    branch feature
    commit
    commit
    checkout main
    merge feature
    commit
```

## Text form

The info word after the language pins one fence, here to the box-drawing form on every sink:

```mermaid text
graph LR
    A[Author] --> B[Render] --> C[Read]
```

## Markup

The `<Mermaid>` widget authors a diagram outside a fence: its source is the default slot's text, or a file read through the page's store, and `render` pins the form as the info word does.

<Mermaid src="docs/design/pipeline.mmd"/>

## Choosing the form

The form is the `diagram-render` cascade property, `Auto`, `Svg` or `Text`, inherited like a text property, so it is set at three grains: an app-wide `<Rule>` in the rule set, a page root's `bx:style="diagram-render=Text"`, or one diagram's fence info word (`mermaid text`) and `<Mermaid render="Text">` prop. `Auto` is the picture on the web; on a terminal a flowchart is text, since it reflows to the columns and selects, and every other type a raster where the session's terminal draws kitty graphics, else text. A build without the svg renderer shows the text form for every mode.
