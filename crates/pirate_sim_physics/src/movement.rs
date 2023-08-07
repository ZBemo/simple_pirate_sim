use bevy_app::prelude::*;
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::prelude::*;
use bevy_time::Time;
use bevy_transform::prelude::*;

use pirate_sim_core::tile_grid::TileStretch;

use super::PhysicsSet;

/// A Ticker, used to keep track of when to actually move a physics component by
/// buffering velocity into its ticker until at least a whole tile has been moved.
///
/// This makes it so that velocities of less than 1 tile per second can be represented in the
/// engine in real time.
///
/// Currently if a component has 0 velocity, its ticker will be reset to 0,0,0. In the future this
/// should be changed so that you can reset your ticker trough a request like RequestResetTicker.
#[derive(Debug, Component, Clone, Copy, Default, Deref, Reflect)]
pub struct Ticker(Vec3);

/// Apply, applies any tickers that have moved at least one tile. This is essentially flushing the
/// MovementTicker buffer.
///
/// This will reset any tickers with a TotalVelocity of 0 to 0,0,0. This may lead to bugs in the
/// future
fn finalize_movement(
    mut phsyics_components: Query<(
        &mut Transform,
        &mut Ticker,
        &super::velocity::RelativeVelocity,
    )>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
) {
    // this will make it so entities only move a tile once an entire tiles worth of movement
    // has been "made", keeping it in a grid based system
    //
    // also converts from grid to tile_stretch

    let delta_time = time.delta_seconds();

    for (mut transform, mut ticker, total_velocity) in phsyics_components.iter_mut() {
        // update ticker, only apply velocity * delta to keep time consistent
        ticker.0 += **total_velocity * delta_time;

        let z_sign = ticker.z.signum();
        let y_sign = ticker.y.signum();
        let x_sign = ticker.x.signum();

        while ticker.z * z_sign >= 1. {
            transform.translation.z += z_sign;
            ticker.0.z -= 1. * z_sign;

            debug_assert!(ticker.z.is_finite() && !ticker.z.is_nan());
            assert!(ticker.z.signum() == z_sign);
        }
        while ticker.y * y_sign >= 1. {
            transform.translation.y += tile_stretch.0 as f32 * y_sign;
            ticker.0.y -= 1. * y_sign;

            debug_assert!(ticker.y.is_finite() && !ticker.y.is_nan());
            assert!(ticker.z.signum() == z_sign);
        }
        while ticker.0.x * x_sign >= 1. {
            transform.translation.x += tile_stretch.1 as f32 * x_sign;
            ticker.0.x -= 1. * x_sign;

            debug_assert!(ticker.x.is_finite() && !ticker.x.is_nan());
            assert!(ticker.z.signum() == z_sign);
        }
    }
}

/// clear tickers when velocity is changed
///
/// TODO: check against last velocity; this will introduce a tiny cost of tracking
/// lastTotalVelocity and lastRelativeVelocity, but probably make it a lot less buggier
fn clear_tickers(
    mut ticker_q: Query<
        (
            &mut Ticker,
            &crate::velocity::RelativeVelocity,
            &crate::velocity::LastRelative,
        ),
        Changed<crate::velocity::RelativeVelocity>,
    >,
) {
    ticker_q.for_each_mut(|(mut t, rv, lrv)| {
        if **rv != **lrv {
            t.0 = Vec3::ZERO;
        }
    });
}

/// A bundle allowing an entity to be moved by the physics system
#[derive(Bundle, Default)]
pub struct MovementBundle {
    velocity_bundle: super::velocity::VelocityBundle,
    ticker: Ticker,
}

pub(super) struct Plugin;

impl bevy_app::Plugin for Plugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            Update,
            (
                finalize_movement
                    .in_set(PhysicsSet::Movement)
                    .after(PhysicsSet::Collision),
                clear_tickers.after(PhysicsSet::Velocity),
            ),
        );
    }
}
