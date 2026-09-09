---
name: phased-plan
description: Template for writing planned documents to be executed either by a single agent or several agents interspersed with reviews.
---

# Phased Plan

Write a phase plan to the plans directory with the provided name, defaulting to master-plan:
- `.agents/plans/write-todos.md`
- `.agents/plans/master-plan.md`

Use a format something like this. It doesn't have to be strictly in this shape, but roughly like this.

```
# Make TODO App

## Next
This section should be updated after every phase

Model: Opus
Effort: High
Phase: 2

## Overview

### Some subheading
## Phases

### Phase 1
...
### Phase 2
...

## Completed

### Phase 1
it went well..
```
