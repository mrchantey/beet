# AI Strategy

Behavioral AI strategies are often described as being somewhere on a spectrum of of tradeoffs.

| Controllable, Predictable, Fast |                |            | Emergent, Creative, Slow |
| ------------------------------- | -------------- | ---------- | ------------------------ |
| Pathfinding Algorithms (A*)     | State Machines | Utility AI | Genetic Algorithms       |
| Hardcoded rules                 | Behavior Trees | GOAP       | Neural Networks          |
|                                 |                |            | LLMs                     |


Just like human decision making we usually want some combination for a given task, for example (this is just illustrative)

| Decision                                       | Time scale      | AI Strategy     | Human Analogy         |
| ---------------------------------------------- | --------------- | --------------- | --------------------- |
| Where should we go for coffee?                 | tens of seconds | LLMs            | Higher-order thinking |
| How should I place my feet while walking?      | seconds         | Utility AI      | Lower-order thinking  |
| Should I drop the cup if its scalding my hand? | milliseconds    | Hardcoded rules | Reflexes              |

## Beets's Approach

The 'go and get coffee' task requires several strategies to work in unison and Beet aims to be a single point of coordination for such behaviors. 

Its also important for these to be made modular, if a member of the community writes a great 'go and get coffee' behavior they should be able to share it for reuse in entirely different contexts, ideally across games *and* robotics where possible.

## AI designed by AI

At some point it may be worth exploring the use of advanced AI models in the design of lower level decision-making systems, for example:
- During development - A model generates a behavior tree for some task, to be fine-tuned by developers and designers.
- Runtime - The enemy AI for an RTS may run an expensive model remotely every few minutes to adjust the *strategy*, but the frame-by-frame decisions are made locally.