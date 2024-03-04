# System Ordering

Each action will run consecutively according to their depth, but for single-frame execution sometimes we need to run something before the parent.

System orderings are accessible via attributes, examples are:
- `before_parent` Useful for GOAP / Utility selectors, allows preparing of score for each child node of a selector.
- `before` - Do something before, ie skip.
- `after` Do something after, ie cleaning up run states.

```rs
tree!{
	<my_action
		before_parent = set_score
		before = cancel_if_elapsed
		after = cleanup
	/>
}
```

Consider the following tree:

```mermaid
graph TB

Node1 --- dot1
Node1 --- Node2
Node2 --- dot2
Node2 --- Node3
dot1[...]
dot2[...]
```

It would produce this system ordering:
```mermaid
graph LR;
	node1 --- node2.before
	node1 --- node1.after 
	node1 --- node3.before_parent 
	node2.before --- node2
	node1.after --- node2
	node3.before_parent --- node2
```

The ordering can also be viewed using [`bevy_mod_debug_dump`](https://crates.io/crates/bevy_mod_debugdump)

![system ordering](../gamai/images/system-ordering.png)