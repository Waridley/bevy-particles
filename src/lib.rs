use std::ops::Mul;
use bevy::ecs::{schedule::ScheduleLabel, query::WorldQuery, system::EntityCommands};
use bevy::prelude::*;
use bevy::utils::{Duration, Instant};
use nanorand::{Rng, WyRand};

pub mod update;
use update::*;

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_systems(PreUpdate, spawn_particles)
			.add_systems(Update, (
				Linear::tick,
				Angular::tick,
				MulScale::tick,
				AddScale::tick,
				TargetScale::tick,
				TargetTransform::tick,
				DynParticleUpdate::tick,
				handle_lifetimes,
			));
	}
}

#[derive(Default, Clone, Bundle)]
pub struct ParticleBundle<M: Material = StandardMaterial> {
	pub mesh_bundle: MaterialMeshBundle<M>,
	pub lifetime: Lifetime,
	pub time_created: TimeCreated,
	pub initial_transform: InitialTransform,
	pub initial_global_transform: InitialGlobalTransform,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct ParticleData<'w> {
	pub mesh: &'w mut Handle<Mesh>,
	// pub material: &'w mut Handle<M>, // Material is generic. Should we just assume StandardMaterial?
	pub transform: &'w mut Transform,
	pub global_transform: &'w mut GlobalTransform,
	pub initial_transform: &'w mut InitialTransform,
	pub initial_global_transform: &'w mut InitialGlobalTransform,
	pub visibility: &'w mut Visibility,
	pub computed_visibility: &'w mut ComputedVisibility,
	pub time_created: &'w mut TimeCreated,
	pub lifetime: &'w mut Lifetime,
}

#[derive(Debug, Clone, Copy, Component, Deref, DerefMut)]
pub struct TimeCreated(pub Instant);

impl Default for TimeCreated {
	fn default() -> Self {
		Self(Instant::now())
	}
}

#[derive(Default, Debug, Clone, Copy, Component, Deref, DerefMut)]
pub struct InitialTransform(pub Transform);

#[derive(Default, Debug, Clone, Copy, Component, Deref, DerefMut)]
pub struct InitialGlobalTransform(pub GlobalTransform);

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub struct Lifetime(pub Duration);

impl Default for Lifetime {
	fn default() -> Self {
		Self(Duration::from_secs(1))
	}
}

pub fn handle_lifetimes(
	mut cmds: Commands,
	mut q: Query<(Entity, &TimeCreated, &Lifetime)>,
	t: Res<Time>,
) {
	for (id, created, lifetime) in &mut q {
		if let Some(update) = t.last_update() {
			if update.duration_since(created.0) > lifetime.0 {
				cmds.entity(id).despawn();
			}
		}
	}
}

pub trait ParticleFactory
where
	for<'w, 's, 'a> Self: FnMut(&'a mut Commands<'w, 's>, &GlobalTransform, TimeCreated) -> EntityCommands<'w, 's, 'a>
		+ Send
		+ Sync
		+ 'static,
{
}
impl<F> ParticleFactory for F where
	for<'w, 's, 'a> F: FnMut(&'a mut Commands<'w, 's>, &GlobalTransform, TimeCreated) -> EntityCommands<'w, 's, 'a>
		+ Send
		+ Sync
		+ 'static
{
}

#[derive(Component)]
pub struct Spewer {
	pub factory: Box<dyn ParticleFactory>,
	pub interval: Duration,
	pub jitter: Duration,
	pub last_spawn: Instant,
	pub use_global_coords: bool,
	pub rng: nanorand::WyRand,
}

#[derive(Default, Bundle)]
pub struct SpewerBundle {
	pub spewer: Spewer,
	pub transform: TransformBundle,
	pub prev_xform: PreviousTransform,
	pub prev_global_xform: PreviousGlobalTransform,
	pub visibility: VisibilityBundle,
}

#[derive(Default, Component, Deref, DerefMut)]
pub struct PreviousTransform(Transform);
#[derive(Default, Component, Deref, DerefMut)]
pub struct PreviousGlobalTransform(GlobalTransform);


fn default_factory<'w, 's, 'a>(
	cmds: &'a mut Commands<'w, 's>,
	_: &GlobalTransform,
	_: TimeCreated,
) -> EntityCommands<'w, 's, 'a> {
	cmds.spawn(ParticleBundle::<StandardMaterial>::default())
}

impl Default for Spewer {
	fn default() -> Self {
		let interval = Duration::from_secs_f32(1.0 / 60.0);
		Self {
			factory: Box::new(default_factory),
			interval,
			jitter: Duration::ZERO,
			last_spawn: Instant::now(),
			use_global_coords: false,
			rng: WyRand::new(),
		}
	}
}

impl Spewer {
	pub fn new(factory: impl ParticleFactory) -> Self {
		Self {
			factory: Box::new(factory),
			..default()
		}
	}

	pub fn seeded(seed: u64) -> Self {
		Self {
			rng: WyRand::new_seed(seed),
			..default()
		}
	}

	pub fn instance(&self, factory: impl ParticleFactory) -> Self {
		Self {
			factory: Box::new(factory),
			interval: self.interval,
			jitter: self.jitter,
			last_spawn: self.last_spawn,
			use_global_coords: self.use_global_coords,
			rng: self.rng.clone(),
		}
	}
}

fn spawn_particles(
	mut cmds: Commands,
	mut q: Query<(Entity, &mut Spewer, &Transform, &GlobalTransform, Option<&mut PreviousTransform>, Option<&mut PreviousGlobalTransform>)>,
	t: Res<Time>,
) {
	let dt = t.delta_seconds();
	for (id, mut spewer, xform, global_xform, mut prev_xform, mut prev_global_xform) in &mut q {
		let Spewer {
			interval,
			jitter,
			use_global_coords,
			ref mut factory,
			ref mut last_spawn,
			ref mut rng,
		} = *spewer;
		let vel = if let Some(prev_global_xform) = &prev_global_xform {
			let prev_xform = prev_global_xform.compute_transform();
			let xform = global_xform.compute_transform();
			Transform {
				translation: (xform.translation - prev_xform.translation) / dt,
				rotation: (xform.rotation - prev_xform.rotation) / dt,
				scale: (xform.scale - prev_xform.scale) / dt,
			}
		} else {
			Transform::IDENTITY
		};
		let interval_secs =  interval.as_secs_f32();
		let step = Transform {
			translation: vel.translation * interval_secs,
			rotation: vel.rotation * interval_secs,
			scale: vel.scale * interval_secs,
		};
		let mut curr_xform = if let Some(prev_global_xform) = &prev_global_xform {
			***prev_global_xform
		} else {
			*global_xform
		};

		let Some(remaining) = t.last_update() else { continue };
		let Some(mut remaining) = remaining.checked_duration_since(*last_spawn) else { continue };

		while remaining >= interval {
			remaining = remaining.saturating_sub(
				interval
					+ Duration::new(
						rng.generate_range(0..=jitter.as_secs()),
						rng.generate_range(0..=jitter.subsec_nanos()),
					),
			);
			*last_spawn += interval;

			let mut particle: EntityCommands =
				(factory)(&mut cmds, &curr_xform, TimeCreated(*last_spawn));
			let particle_id = particle.id();
			if !use_global_coords {
				cmds.entity(id).add_child(particle_id);
			}
			let tmp = curr_xform.compute_transform();
			curr_xform = Transform {
				translation: tmp.translation + step.translation,
				rotation: tmp.rotation + step.rotation,
				scale: tmp.scale + step.scale,
			}.into();
		}
		if let Some(mut prev_xform) = prev_xform {
			if **prev_xform != *xform {
				**prev_xform = *xform;
			}
		}
		if let Some(mut prev_global_xform) = prev_global_xform {
			if **prev_global_xform != *global_xform {
				**prev_global_xform = *global_xform;
			}
		}
	}
}
