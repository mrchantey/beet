---
name: improving-docs
description: Audit, restructure, or incrementally improve an existing body of documentation using Diátaxis. Use when the task is to assess docs, untangle mixed-up content, or raise quality across a project rather than write a single new piece. Covers the iterative workflow and how to judge documentation quality. Load the `diataxis` skill to pick a mode, and the writing-* skills to act on each piece.
---

# Improving documentation

Diátaxis is a compass, not a map. Do not impose a finished structure on a project up
front. Good structure grows from within, the way an organism grows, through many
small improvements. The aim is documentation that is *complete* at every stage (useful
and appropriate as it stands) even if it is never *finished*.

## The workflow

Work one small piece at a time, in a tight loop:

1. **Choose something.** Any paragraph, section, or page. Start where you are.
2. **Assess it critically.** Which mode is this trying to be? Use the compass in the
   `diataxis` skill. Is it serving its user's actual need, or fighting itself by
   mixing modes?
3. **Decide one action.** Identify a single improvement that would help right now.
   Resist the urge to plan a grand restructure.
4. **Do it and publish.** Make that one change and ship it. Every step in the right
   direction is worth publishing immediately.

Then pick the next thing. Over many cycles the documentation takes on Diátaxis shape
on its own, without a big migration.

Avoid: creating empty section templates, attempting a full reorganization in one go,
or accumulating large batches of unpublished work.

## Spotting problems

The most common defect is mixed modes in one document. Look for:
- A tutorial or how-to guide that keeps stopping to explain.
- Reference that tells the reader what to do.
- Explanation that drifts into step-by-step procedure.

When you find mixed content, split it. Move each part to the mode it belongs in and
link between them. The fix is almost always separation, not rewriting.

## Judging quality

Quality has two layers, and the deep one depends on the functional one.

**Functional quality** is objective and testable against the subject:
accuracy, completeness, consistency, usefulness, precision. These are independent
constraints. Failures are obvious to users. Get these right first.

**Deep quality** is subjective and recognized by feel: flow, fitting human needs,
anticipating the user, beauty, feeling good to use. These qualities are
interdependent and they are *conditional* on functional quality, ie you cannot make
inaccurate, inconsistent docs feel good. Good documentation feels good to use, and
the satisfaction is the signal.

Diátaxis cannot guarantee quality. By keeping each piece in a single user-centered
mode, it lays down the conditions under which deep quality becomes possible.
