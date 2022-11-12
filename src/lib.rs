use bevy::ecs::{query::WorldQuery, system::EntityCommands};
use bevy::prelude::*;
use bevy::utils::{Duration, Instant};
use rand::Rng;
use std::sync::Arc;

pub mod update;
use update::*;

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
	fn build(&self, app: &mut App) {
		app.add_system(Linear::tick)
			.add_system(Angular::tick)
			.add_system(MulScale::tick)
			.add_system(AddScale::tick)
			.add_system(TargetScale::tick)
			.add_system(MulTransform::tick)
			.add_system(DynParticleMovement::tick)
			.add_system_to_stage(CoreStage::PreUpdate, spawn_particles)
			.add_system(handle_lifetimes);
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

#[derive(Debug, Clone, Component)]
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
	for<'w, 's, 'a> Self: Fn(&'a mut Commands<'w, 's>, &GlobalTransform, TimeCreated) -> EntityCommands<'w, 's, 'a>
		+ Send
		+ Sync
		+ 'static,
{
}
impl<F> ParticleFactory for F where
	for<'w, 's, 'a> F: Fn(&'a mut Commands<'w, 's>, &GlobalTransform, TimeCreated) -> EntityCommands<'w, 's, 'a>
		+ Send
		+ Sync
		+ 'static
{
}

#[derive(Clone, Component)]
pub struct Spewer {
	pub factory: Arc<dyn ParticleFactory>,
	pub interval: Duration,
	pub jitter: Duration,
	pub last_spawn: Instant,
	pub global_coords: bool,
}

#[derive(Default, Bundle)]
pub struct SpewerBundle {
	pub spewer: Spewer,
	pub transform: TransformBundle,
	pub visibility: VisibilityBundle,
}

impl Default for Spewer {
	fn default() -> Self {
		let interval = Duration::from_secs_f32(1.0 / 60.0);
		Self {
			factory: Arc::new(|cmds: &mut Commands, _, _| {
				cmds.spawn(ParticleBundle::<StandardMaterial>::default())
			}),
			interval,
			jitter: Duration::ZERO,
			last_spawn: Instant::now(),
			global_coords: false,
		}
	}
}

fn spawn_particles(
	mut cmds: Commands,
	mut q: Query<(Entity, &mut Spewer, &GlobalTransform)>,
	t: Res<Time>,
) {
	for (id, mut spewer, xform) in &mut q {
		let Spewer {
			interval,
			jitter,
			global_coords,
			ref factory,
			ref mut last_spawn,
		} = *spewer;

		let Some(remaining) = t.last_update() else { continue };
		let Some(mut remaining) = remaining.checked_duration_since(*last_spawn) else { continue };

		while remaining >= interval {
			remaining = remaining
				.saturating_sub(interval + rand::thread_rng().gen_range(Duration::ZERO..=jitter));
			*last_spawn = *last_spawn + interval;

			let mut particle: EntityCommands =
				(factory)(&mut cmds, xform, TimeCreated(*last_spawn));
			let particle_id = particle.id();
			if !global_coords {
				cmds.entity(id).add_child(particle_id);
			} else {
				particle.insert(xform.compute_transform());
			}
		}
		// if spawn_timer.tick(t.delta()).finished() {
		// 	let jitter = rand::thread_rng().gen_range(Duration::ZERO..=jitter);
		// 	spawn_timer.set_duration(interval + jitter);
		// 	spawn_timer.reset();
		// }
	}
}
