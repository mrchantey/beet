---
name: writing-reference
description: Write Diátaxis reference material, ie information-oriented documentation that neutrally describes a product's machinery (APIs, functions, classes, commands, options, config) for users to consult while working. Use when the reader needs accurate facts to look up, not steps to follow (how-to guide) or background to understand (explanation). Load the `diataxis` skill first if unsure which mode applies.
---

# Writing reference

Reference is a technical description of the machinery and how to operate it. It is **information-oriented**: propositional, factual knowledge that users *consult* while they work, not prose they read end to end. Good reference gives users truth and certainty, a firm platform to stand on. Like a map describing territory, it lets a user understand the product without reading the source.

## How to write it

1. **Describe, and only describe.** This is the one rule. Be austere, neutral, objective, factual. State what the thing is, its parameters, its return values, its options, its limitations, its errors. Do not instruct, explain, persuade, or opine. When the user needs those, link to a how-to guide, tutorial, or explanation.
2. **Mirror the product's structure.** Let the shape of the documentation follow the shape of the code or system, so a user can navigate both in parallel. Reference for module X lives in a predictable place relative to module Y.
3. **Be consistent and predictable.** Adopt standard patterns and formats and repeat them everywhere. Put information where users expect to find it, in a format they already know. Resist stylistic flourish; predictability is the feature.
4. **State facts directly.** "Sub-commands are: a, b, c, d." List features, commands, options, flags, limitations, and error messages. Include necessary warnings. Use plain declarative sentences.
5. **Provide examples.** Short usage examples illustrate the facts and show context, without crossing over into instruction.

## What to avoid

- Letting instruction or explanation creep in. A "describe" that becomes a "do this" has stopped being reference.
- Assuming auto-generated docs are sufficient. Generated signatures are a skeleton; the descriptive prose still has to be written and curated.
- Mixing in marketing claims or recipes alongside the factual material.

## Accuracy

Reference is the mode where accuracy matters most, because users trust it as ground truth and act on it without verifying. Keep it correct and current with the product.
