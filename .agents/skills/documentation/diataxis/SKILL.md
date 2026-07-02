---
name: diataxis
description: Entry point for writing or improving software documentation using the Diátaxis framework. Use this first whenever you are about to write, restructure, or assess docs (README sections, guides, API docs, conceptual articles). It identifies which of the four documentation modes fits the task, then routes you to the matching authoring skill.
---

# Diátaxis

Diátaxis is a framework for the structure of technical documentation. It identifies four distinct documentation modes, each serving a different user need. Mixing modes in a single document is the most common cause of poor docs, so the first job is always to decide which mode you are writing in.

## The four modes

| Mode | Serves | Orientation | The user is... |
|------|--------|-------------|----------------|
| **Tutorial** | Learning | action + acquisition | studying, being taught a skill |
| **How-to guide** | A task | action + application | working, solving a specific problem |
| **Reference** | Looking up | cognition + application | working, needs facts to consult |
| **Explanation** | Understanding | cognition + acquisition | studying, wants the bigger picture |

Two axes separate them:
- **Action vs cognition**: is the content about *doing* (practical steps) or *thinking* (knowledge)?
- **Acquisition vs application**: is the user *studying* (learning something new) or *working* (applying what they know)?

## The compass: choosing a mode

Ask two questions about the content:

1. Is it about **action** or **cognition**?
2. Does it serve **acquisition** (study) or **application** (work)?

| | Action | Cognition |
|---|---|---|
| **Acquisition** | Tutorial | Explanation |
| **Application** | How-to guide | Reference |

Rules of thumb:
- "Teach a beginner this skill from scratch" -> **Tutorial**
- "Help a competent user accomplish a specific goal" -> **How-to guide**
- "Let a user look up exact facts while they work" -> **Reference**
- "Help a user understand why/how something is, the context and alternatives" -> **Explanation**

Apply the compass at every level, from a whole document down to a single paragraph. A symptom that you have picked the wrong mode is content that feels like it is fighting itself, ie a tutorial that keeps stopping to explain, or reference that keeps telling the reader what to do.

## Route to the right skill

Once you know the mode, load the matching skill and follow it:

- Tutorial -> `writing-tutorials`
- How-to guide -> `writing-how-to-guides`
- Reference -> `writing-reference`
- Explanation -> `writing-explanation`

If the task is auditing, restructuring, or improving existing docs rather than writing one new piece, load `improving-docs`.

## Working principle

Do not try to design the whole documentation structure up front. Good structure emerges from incremental improvement: pick one piece, make it serve its single mode better, publish, repeat. See `improving-docs` for the full workflow.
