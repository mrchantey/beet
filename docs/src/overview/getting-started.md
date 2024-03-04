# Getting Started

A fundamental concept of Beet is behavior trees. Consider the following diagram for Billy the Bee 🐝 

```mermaid
graph TB;
    A-->B;
		A-->C;

A[sequence]
B((has_energy))
C[wander]
```