use bevy::prelude::*;

use super::TileStretch;

/// A Ticker, used to keep track of when to actually move a physics component by
/// buffering velocity into its ticker until at least a whole tile has been moved.
///
/// This makes it so that velocities of less than 1 tile per second can be represented in the
/// engine in real time.
///
/// Currently if a component has 0 velocity, its ticker will be reset to 0,0,0. In the future this
/// should be changed so that you can reset your ticker trough a request like RequestResetTicker.
///
/// As this Ticker is meant to be wholly managed by the physics engine, it is not public, and must
/// be instantiated trough a Bundle like [`PhysicsComponentBase`]
#[derive(Debug, Component, Clone, Copy, Default, Deref, Reflect)]
pub struct Ticker(Vec3);

/// Apply, applies any tickers that have moved at least one tile. This is essentially flushing the
/// MovementTicker buffer.
///
/// This will reset any tickers with a TotalVelocity of 0 to 0,0,0. This may lead to bugs in the
/// future
///
/// This will also avoid moving two [`ColliderType::Solid`] into each other by lessening their
/// velocity.
fn finalize_movement(
    mut phsyics_components: Query<(
        &mut Transform,
        &mut Ticker,
        &super::velocity::RelativeVelocity,
        Option<&super::collider::Collider>,
    )>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
) {
    // this will make it so entities only move a tile once an entire tiles worth of movement
    // has been "made", keeping it in a grid based system
    //
    // also converts to 32x32

    for (mut transform, mut ticker, total_velocity, _collider) in phsyics_components.iter_mut() {
        // update ticker, only apply velocity * delta to keep time consistent
        ticker.0 += total_velocity.0 * time.delta_seconds();

        debug!("updating with ticker {}", ticker.0);

        while ticker.0.z.abs() >= 1. {
            transform.translation.z += ticker.0.z.signum();
            ticker.0.z -= 1. * ticker.0.z.signum();
        }
        while ticker.0.y.abs() >= 1. {
            transform.translation.y += tile_stretch.y as f32 * ticker.0.y.signum();
            ticker.0.y -= 1. * ticker.0.y.signum();
        }
        while ticker.0.x.abs() >= 1. {
            transform.translation.x += tile_stretch.x as f32 * ticker.0.x.signum();
            ticker.0.x -= 1. * ticker.0.x.signum();
        }

        // this might break things in the future!
        // if total_velocity is 0 reset ticker to 0
        // this probably does not belong in this system. maybe in its own system?
        if **total_velocity == Vec3::ZERO {
            ticker.0 = Vec3::ZERO;
        }
    }
}

pub(super) struct Plugin();

impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(
            finalize_movement
                .in_set(super::PhysicsSet::FinalizeMovement)
                .after(super::PhysicsSet::FinalizeCollision),
        );
    }
}
