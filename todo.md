# General code cleanliness
 - [x] use IVec3 in places where it makes sense (like everywhere)
 - [x] split physics.rs into multiple modules
    - [x] split out collision
    - [x] split out velocity calculation to its own module
    - [x] consider splitting out movement
 - [ ] Return an iterator from [`find_and_resolve_conflicts`](./src/physics/collider.rs:244)
 - [ ] Strongly type TileSpace
 - [ ] chunk out startup systems, probably using game states, 
 allowing doing startups after necessary resources are set up more easily
 - [x] change query function parameters to $NAME_q. will make code far more readable
 - [ ] set up cargo clippy and lint 
 - [ ] start checking docs for correctness
 - [ ] slim down bevy's DefaultPlugins. this might already be taken care of by slimming down features

# Current 
Physics-collision and resolution 
 - [x] Chunk up into functions
 - [ ] figure out all different modes of resolution
    - [x] clamp velocity - the preferred mode of resolution, essentially remove velocity from the object to stop it from moving into colliders
    - [ ] apply velocity - a second choice, "push" the collider out of the collision zone
       - Make sure this pushes in a reasonable direction. No clipping under floors, flight hacks, etc
       - figure out how to implement this.  
 - [ ] loop until all collisions for single frame resolved. Currently, with only clamping velocity, this should not be an issue.
 - [ ] "unclipping" system for colliders. Push them out of places that they shouldn't be.
 - [ ] Figure out when to send collision events, and what to include
   - [x] Entity collision events
   - [ ] tile collision events
   - right now I'm thinking other entities it collided with, where it was going to collide, and if a resolution was needed (Potentially per-entity)
     For example, if you collide into one sensor collider, which also exists on a wall, you needed resolution with the wall, but merely collided with the sensor.


# Big features
 - [ ] "full" tile physics engine (roughly in order)
  - [x] propogate velocities
  - [x] Collision checking
   - [x] figure out what to do on collision. possible have option on how to handle it in collider, or based on other components
  - [x] Collision resolution
  - [x] Collision event system
  - [ ] "take" velocity from floor
  - [ ] finalize, and probably re-architect continuous velocity
  - [ ] fine tune gravity
 - [x] Information display setup for gui, easier development
 - [ ] Player and AI interaction 
    - [ ] ladders
    - [ ] guns
    - [ ] cannons
    - [ ] swords
    - [ ] player and AI view and memory simulation
 - [ ] tile rendering and sprite features
    - [ ] Custom spritesheet somehow
    - [ ] cull sprites on other layers.
        - [ ] add camera viewports for events above/below 
            - [ ]  rework code to be aware of >1 camera
        - [ ] let player look around
    - [ ] spritesheet processing
     - [ ] probably needs a mix of preprocessing and processing on tile update
     - [ ] preprocessing
     - [ ] tile update processing
    - [ ] dynamically update spritesheet colors
    - [ ] dynamically update tile sprites
 - [ ] ships
    - requires certain spritesheet features
    - [ ] ship rotate along z axis with half-steps
    - [ ] steering
    - [ ] holes/sinking
    - [ ] generated treasure?
 - [ ] enemy and friendly AI
    - [ ] teams
    - [ ] AIs have different strengths
    - [ ] AI self-preservation systems
    - [ ] AI gives and takes commands
    - [ ] should be able to use same interactions as player

# Upgrades

## Physics
 - [x] use a configurable constraint system for colliders instead of like 6 different types
