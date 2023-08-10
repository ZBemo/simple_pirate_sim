An in-the-works tile-based top-down pirate simulator with procedural AI.

See [./todo.md](./todo.md) for planned and added features.

# Building

Simple profiling: 

`cargo run --release --no-default-features --features bevy/trace_chrome` You can swap out other bevy profiling features if desired 

Run with only fps counter:
`cargo run --release --no-default-features --features=pirate_sim_game/fps-diagnostics`

Build for full release 
`cargo build --release --no-default-features`
