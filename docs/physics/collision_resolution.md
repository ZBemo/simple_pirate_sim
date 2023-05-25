A file for architecting collision resolution from a high level down

Start by getting the planes passed through by each colliding entity

Check these planes against constraint.

Use event system to keep track of eminent collisions, not just stored in collider.

First, any entities in a collision that do not move will not be considered for resolution.

Any entity that can move will have its velocity clamped down to the minimum amount it can move before colliding.
Any entity that can move but is not moving, but still in conflict will have a velocity applied to move it out of the collision.

Calculate entity movement direction. (location - collision location)

Before moving, make sure there won't be collisions in its new location. If so, move it elsewhere. 
