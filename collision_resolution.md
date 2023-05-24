A file for architecthing collision resolution from a high level down

start by getting the planes passed through by each colliding entity

check these planes against constraint.

use event system to keep track of immenent collisions, not just stored in collider.

first, any entities in a collision that do not move will not be considered for resolution.

second, sort entities by location, in order to make collision resolution more deterministic.

calculate entity movement direction. (location - collision location)
