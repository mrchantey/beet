---
name: writing-how-to-guides
description: Write a Diátaxis how-to guide, ie goal-oriented documentation that directs a competent user through the steps to accomplish a specific real-world task. Use when the reader already knows the basics and needs to get something done, not when teaching a beginner (tutorial) or describing the machinery (reference). Load the `diataxis` skill first if unsure which mode applies.
---

# Writing how-to guides

A how-to guide is a recipe: directions that take a user through the steps to achieve a specific end. It is **goal-oriented** and serves users who are *working*, not studying. It assumes competence. Its job is to answer a real question of the form "How do I do X?".

A good how-to guide is the clearest demonstration of what your product can actually do for someone.

## How to write one

1. **Address a real problem, from the user's side.** Frame it around what the user wants to achieve ("how to calibrate the radar array", "how to configure reconnection policies"), not around the machinery. Describing tool mechanics for their own sake is mostly useless to someone who already has basic competence.
2. **Keep one goal in focus.** A how-to guide is about a task with a practical goal. Maintain focus on that goal. Do not wander into teaching or explanation.
3. **Provide an executable sequence.** Lay out the steps as a logical contract: given situation X, do these things, and you will reach result Y. Steps include judgment and decisions, not just mechanical commands.
4. **Sequence with real-world logic.** Order the steps the way the work actually flows. Anticipate what the user needs next and present it exactly when needed, so the guide seems to read the user's mind. This is *flow*.
5. **Stay adaptable.** Real users have varying situations. Address the task in a way they can adapt, rather than a single brittle path. Use conditional imperatives: "If you want x, do y."
6. **Be usefully incomplete.** Prefer practical usability over completeness. Start and stop at sensible points; the user slots the guide into their own work. Omit anything not needed to reach the goal.

## Naming

Name it by exactly what it lets the user do. The title should answer "how to...".

- Good: "How to integrate application performance monitoring"
- Ambiguous: "Integrating application performance monitoring" (do, or decide?)
- Useless: "Application performance monitoring" (a topic, not a task)

Scope must be specific and bounded. "How to build a web application" is too broad to be a how-to guide; it has no defined endpoint.

## Language patterns

- Open with "This guide shows you how to...".
- Use conditional imperatives: "If you want x, do y".
- Point elsewhere for detail: "Refer to the x reference for the full options" rather than reproducing reference material inline.

## What to avoid

- Including explanation or teaching material.
- Defining the guide around tool functions instead of user problems.
- Reaching for reference-level completeness when focus matters more.
- Titles that hide the guide's real scope or intent.
