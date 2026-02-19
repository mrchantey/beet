- tool.rs macro has a lot of duplication when generating the IntoToolHandler, refactor into helper function.



## Entity tool chaining

Chaining tools between entities at runtime.

This is ephemeral, create a tool chain but it isnt actually stored as a component.

I guess its a matter of querying for the render tool


```
query.single.unwrap_err()
renderer not found
```

## Card refactor


- RenderToolMarker
- RenderRequest
