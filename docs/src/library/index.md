# The Beet Library

Beet relies on two components to run. both are `sparse_set` and added/removed as the run state of a node changes.

## `Running`

A marker component indicating this node in the graph is currently running. The root node starts with this component and its usually added to children by parents to indicate "you're up!"

## `RunResult` 

Any action can decide to end the run state for its node by inserting a `RunResult`. Parents will usually listen for this change and somehow react to it, depending on whether it was `RunResult::Success` or `RunResult::Failure`.

Thats it! The goal is for everything else to be optional, although the library still has some cleaning up to do.

