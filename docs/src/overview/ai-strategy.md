# AI Strategy

## The Landscape

Behavioral AI strategies are often described as being somewhere  on a spectrum of of tradeoffs.

| Controllable, Predictable, Fast |                |            | Emergent, Creative, Slow |
| ------------------------------- | -------------- | ---------- | ------------------------ |
| A* Navigation                   | State Machines | Utility AI | Genetic Algorithms       |
| Hardcoded rules                 | Behavior Trees | GOAP       | Neural Networks          |
|                                 |                |            | LLMs                     |


Just like human decision making we usually want some combination for a given task, for example: 

| Decision                                       | Time scale      | AI Strategy     | Human Analogy          |
| ---------------------------------------------- | --------------- | --------------- | ---------------------- |
| Where should we go for coffee?                 | tens of seconds | LLMs            | Neo cortex             |
| How should I place my feet while walking?      | seconds         | Utility AI      | Lower brain            |
| Should I drop the cup if its scalding my hand? | milliseconds    | Hardcoded rules | Central Nervous System |

## Beets's Approach

The 'go and get coffee' task requires several strategies to work in unison and Beet aims to be a single point of coordination for such tasks. Its also important for these to be made modular, if a member of the community writes a great 'go and get coffee' script they should be able to share it for reuse in entirely different contexts, ideally across games *and* robotics where possible.

## The future

Its possible at some point in the future we'll 

compile time checked
runtime readjust strategy every 5 mins rts