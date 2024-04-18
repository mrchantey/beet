# Robotics

Bevy is a remarkably modular engine. Because it seperates the underlying `bevy_ecs` crate from rendering etc it can run on tiny microcontrollers like the ESP32.
<iframe width="1280" height="720" src="https://www.youtube.com/embed/R-q5iJ98X40" title="Bevy + Beet  + ESP32 Hello World" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

## But why use a game engine?

Just because we *can* run bevy on a microntroller, should we? I think so and here's a few reasons why:

- Simulation: We get a simulation environment out of the box
- Ecosystem: Share and reuse libraries like netcode, navigation or behavior selection
- Animation: Bringing robotics tooling closer to skilled animators is an important step toward quality human interaction
- ECS beyond games: Its a really nice architectural pattern, and performance is also critical in robotics