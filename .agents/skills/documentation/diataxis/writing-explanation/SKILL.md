---
name: writing-explanation
description: Write Diátaxis explanation, ie understanding-oriented documentation that discusses a subject to deepen the reader's grasp of it, covering context, reasons, design decisions, history, and alternatives. Use when the reader wants to understand why and how something is, not to perform a task (how-to guide) or look up facts (reference). Load the `diataxis` skill first if unsure which mode applies.
---

# Writing explanation

Explanation is a discursive treatment of a subject that permits reflection. It is **understanding-oriented**. It answers the question "Can you tell me about...?". Explanation is read away from the product, at leisure, to build the web of understanding that holds everything else together. Understanding does not come from explanation alone, but explanation is what weaves it.

It is the least urgent of the four modes and the easiest to neglect, but it is what turns a user who can follow steps into one who can reason about the system.

## How to write it

1. **Discuss the subject, do not instruct.** Talk *about* a topic. A useful test: the title carries an implicit "about", as in "About user authentication". If you find yourself writing steps to perform, that content belongs in a how-to guide.
2. **Make connections.** Link ideas together, within the topic and out to others. The value of explanation is in the relationships it draws between things.
3. **Provide context and background.** Explain the reasons behind design decisions, the historical accidents, the technical constraints. "The reason for X is that historically Y..."
4. **Consider alternatives and admit opinion.** Explanation can and must weigh counter-examples, multiple approaches, and trade-offs. "W is better than Z because...". Acknowledging perspective and judgment is appropriate here in a way it is not in the other three modes.
5. **Bound the scope.** Keep the discussion to its subject. Do not let it absorb instructions or detailed technical description that belong elsewhere.

## Language patterns

- "The reason for X is that historically, Y..."
- "W is better than Z, because..."
- "An X interacts with Y as follows..."

## What to avoid

- Scattering explanation through tutorials, how-to guides, and reference instead of giving it its own home. Each stray paragraph of "why" in another mode dilutes both.
- Slipping into instructions or step-by-step procedure.
- Reproducing reference-style technical description in place of discussion.
