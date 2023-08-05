# Bugs 
 - [ ] when to right of collider pressing `daaz` in sequence will step onto that collider when it shouldn't - tilecasting error?
 - [ ] disabling default features doesn't disable bevy inspector egui

# Refactor
 - [x] Get rid of crates/bevy; have every crate use bevy w/ features it needs or specific crates that it needs
      This should end up being less annoying overall as each crate will only need to enable features it uses, 
      and we won't need to wait for every bevy crate to compile before compiling our crates, leading to significant comptime speedups
 - [ ] Move pirate_sim_game out to the outermost crate - no reason to just have a pass-through
 - [ ] get or empty should be get or ZERO

# Performance
 - [x] split out into multiple crates?
 - [ ] mark public functions that should be inlined #[inline]
 - [ ] look into using Query::for_each instead of `for _ in query`
 - [ ] test on windows to check if wslg is source of significant frame loss
 - [ ] Put change detection back into total velocity propagation
 - [ ] Use trace!() less often
 - [ ] put tracing spans in perf critical systems

# General code cleanliness
 - [x] use IVec3 in places where it makes sense (like everywhere)
 - [x] split physics.rs into multiple modules
    - [x] split out collision
    - [x] split out velocity calculation to its own module
    - [x] consider splitting out movement
 - [x] change query function parameters to $NAME_q. will make code far more readable
 - [x] update`ConsoleCommand`s to pass in closures instead of trait-structs
 - [x] set up cargo clippy and lint 
 - [x] consider splitting long systems into piped systems where useful
 - [x] Look at updating the string type for console parsing to pare down on clones, consider a COW - not necessary as we want ownership
 - [ ] change {Relative,Total}Velocity to {Relative,Total}
 - [ ]  use a macro that will either debug_assert!() or log an error/warn based on whether debug asserts are enabled
 - [ ] consider changing console::io::ConsoleOpen to an enum
 - [ ] Return an iterator from [`find_and_resolve_conflicts`](./src/physics/collider.rs:244)
 - [ ] Strongly type TileSpace coordinates
 - [ ] chunk out startup systems, probably using game states, 
 allowing doing startups after necessary resources are set up more easily
 - [ ] start checking docs for correctness
 - [ ] ~~slim down bevy's DefaultPlugins. this might already be taken care of by slimming down features~~

# Current 
Tests
 - [x] TileGrid
 - [x] Collisions 
 - [x] TotalVelocity propagation
 - [ ] Velocity (kind of tested by collisions)
Physics-collision and resolution 
 - [x] Chunk up into functions
 - [x] change to tile cast for predicting collison for better accuracy
 - [ ] figure out all different modes of resolution
    - [x] clamp velocity - the preferred mode of resolution, essentially remove velocity from the object to stop it from moving into colliders
    - [ ] apply velocity - a second choice, "push" the collider out of the collision zone
       - Make sure this pushes in a reasonable direction. No clipping under floors, flight hacks, etc
       - figure out how to implement this.  
 - [ ] Figure out when to send collision events, and what to include
   - [ ] Entity collision events
   - [ ] switch to tile collision event - should be more ergonomic
   - [ ] mix of both tile & entity collision events - or entity collision events with Collision Resource?
         CollisionMap resource would be Hasmap<IVec3, EntityCollision> or similar, allows lookup from Collider
         or store collision information on collider, which would be more ecs friendly

# Big features
 - [ ] "full" tile physics engine (roughly in order)
   - [x] propogate velocities
   - [x] Collision checking
   - [x] figure out what to do on collision. possible have option on how to handle it in collider, or based on other components
   - [x] Collision resolution
   - [x] "take" velocity from floor
   - [ ] Collision event system
   - [ ] finalize, and probably re-architect continuous velocity
   - [ ] fine tune gravity
 - [ ] Player Input
   - [ ] Allow multiple movement goals, add them all together to get final movement goal - this doesn't work completely right
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
