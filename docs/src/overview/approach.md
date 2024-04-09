# Approach

Behavioral AI is often described as a spectrum of tradeoffs between control and emergence.

| Very Controllable        |                |            | Very Emergent      |
| ------------------------ | -------------- | ---------- | ------------------ |
| Hardcoded / Unstructured | State Machines | Utility AI | Genetic Algorithms |
|                          | Behavior Trees | GOAP       | Neural Networks    |
|                          |                |            | LLMs               |

Usually what we want is some mixture, much like human faculties. This is often associated with the amount of time/compute required to make the decision:

| Decision                          | Time scale      | AI Strategy    | Human Analogy          |
| --------------------------------- | --------------- | -------------- | ---------------------- |
| Ouch hot stove, retract hand now! | milliseconds    | Hardcoded      | Central Nervous System |
| Should I have a water or a coke?  | seconds         | Utility Scorer | Lower brain            |
| What should I do today?           | tens of seconds | LLM            | Neo cortex             |

Beet is built with this flexibility in mind, it is based on a couple of principles, upon which many different AI strategies can be employed.