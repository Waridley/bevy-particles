use bevy::ecs::{query::WorldQuery, system::EntityCommands};
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
			InterpTransform::tick,
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
	pub global_coords: bool,
	pub rng: nanorand::WyRand,
}

#[derive(Default, Bundle)]
pub struct SpewerBundle {
	pub spewer: Spewer,
	pub transform: TransformBundle,
	pub visibility: VisibilityBundle,
}

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
			global_coords: false,
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
			global_coords: self.global_coords,
			rng: self.rng.clone(),
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
			ref mut factory,
			ref mut last_spawn,
			ref mut rng,
		} = *spewer;

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
				(factory)(&mut cmds, xform, TimeCreated(*last_spawn));
			let particle_id = particle.id();
			if !global_coords {
				cmds.entity(id).add_child(particle_id);
			} else {
				particle.insert(xform.compute_transform());
			}
		}
	}
}
