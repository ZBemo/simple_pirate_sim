# General code cleanliness
 - [x] use IVec3 in places where it makes sense (like everywhere)
 - [x] split physics.rs into multiple modules
    - [x] split out collision
    - [x] split out velocity calculation to its own module
    - [x] consider splitting out movement
- [x] change query function parameters to $NAME_q. will make code far more readable
 - [ ] consider changing console::io::ConsoleOpen to an enum
 - [ ] Return an iterator from [`find_and_resolve_conflicts`](./src/physics/collider.rs:244)
 - [ ] Strongly type TileSpace
 - [ ] chunk out startup systems, probably using game states, 
 allowing doing startups after necessary resources are set up more easily
 - [ ] consider splitting long systems into piped systems where useful
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
 - [ ] conflict resolution should probably run on collision events, for better encapsulation 
 - [ ] Figure out when to send collision events, and what to include
   - [x] Entity collision events
   - [ ] potentially have an asset with all of the collisions that occur in a frame stored for more advanced use.

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
 - [ ] Player Input
   - [x] Allow multiple movement goals, add them all together to get final movement goal
   - [ ] Allow "freeze time" actions, which pause time while player aims etc along with "normal time" actions which should auto target
   - [ ] set up rebinding
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
 - [ ] Dev console - mostly for testing
    - [x] Basic command input/output and registration
    - [ ] implement a standard library of useful commands
    - [ ] Redirect logging output to console if enabled
    - [ ] Dev console variables and/or $() syntax so that you can use command output as arguments to other commands

# 0.2 Milestones
We'll change version to  0.2 once all of these are satisfied. They'll be pretty vague and abstract milestones
 - [ ] Full playability - NPCs and Human can at the very least steer ships and fire at each other, etc
 - [ ] Ships - ships should be steerable, and have canon interactions, etc working. May not have sinking or destructibility implemented yet
 - [ ] Menus - stretch goal - there should be a main menu, way to quit game, etc

# Upgrades


## Physics
 - [x] use a configurable constraint system for colliders instead of like 6 different types
