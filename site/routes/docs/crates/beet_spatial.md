+++
title = "beet_spatial"
+++

# beet_spatial

`beet_spatial` is a library of actions for moving things around a Bevy world. It builds straight on [beet_action](/docs/crates/beet_action), so spatial behavior composes with the same control-flow primitives as everything else: a steering behavior is an action, and a behavior tree can mix it with logging, timers or agent calls.

It covers the usual layers of motion:

- **Movement**: translate, hover, rotate-to-velocity and force integration.
- **Steering**: seek, wander, arrive, align, separate and cohere.
- **Inverse kinematics** and **procedural animation**.
- **Robotics** helpers.

Add `BeetSpatialPlugins` to register the actions and their tick schedule, then spawn the ones you need as components on a moving entity.
