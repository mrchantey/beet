---
name: phased-plan
description: Template for writing planned documents to be executed either by a single agent or several agents interspersed with reviews.
---

# Phased Plan

Write a phase plan to the plans directory with the provided name, defaulting to master-plan:
- `.agents/plans/write-todos.md`
- `.agents/plans/master-plan.md`

## Verbs

Every phase heading carries exactly one verb, usually `build`:

- `settle`: define words, rules and principles. The deliverable is documentation (site docs, READMEs, module docs) and this plan edited.
- `investigate`: find out. The deliverable is a report under `.agents/reports/` and this plan edited. An investigation that spans many crates or systems is fanned out: the phase's agent spawns one sub-agent per crate or system against one rubric and merges their findings itself.
- `build`: code, tests and deletions.

A `settle` or `investigate` phase ends by editing the plan: insert sub-phases (`11.1`, `11.2`, ..) under the phases its outcome changes, each with its own verb and a spec a fresh agent can execute, and no more than necessary. It never spawns a separate plan unless the plan says so.

## Format

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

### Phase 1 (settle)
...
### Phase 2 (build)
...
#### Phase 2.1 (build)
inserted by phase 1, the rename its glossary decided
### Phase 3 (investigate)
...

## Completed

### Phase 1
it went well..
```
