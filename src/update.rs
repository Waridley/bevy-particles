use bevy::prelude::*;

use super::*;

#[derive(Component)]
pub struct Linear {
	pub velocity: Vec3,
}
impl Linear {
	pub fn tick(
		mut q: Query<(&Self, &mut Transform, &InitialTransform, &TimeCreated)>,
		t: Res<Time>,
	) {
		for (item, mut xform, init_xform, t_created) in &mut q {
			xform.translation = init_xform.translation
				+ (item.velocity
					* t.last_update()
						.unwrap()
						.duration_since(t_created.0)
						.as_secs_f32());
		}
	}
}

#[derive(Component)]
pub struct Angular {
	pub velocity: Quat,
}
impl Angular {
	pub fn tick(mut q: Query<(&Self, &mut Transform)>, t: Res<Time>) {
		for (item, mut xform) in &mut q {
			xform.rotation = xform.rotation.slerp(item.velocity, t.delta_seconds())
		}
	}
}

#[derive(Component)]
pub struct MulScale {
	pub scale: Vec3,
}
impl MulScale {
	pub fn tick(mut q: Query<(&Self, &mut Transform)>, t: Res<Time>) {
		for (item, mut xform) in &mut q {
			xform.scale *= Vec3::ONE.lerp(item.scale, t.delta_seconds())
		}
	}
}

#[derive(Component)]
pub struct AddScale {
	pub scale: Vec3,
}
impl AddScale {
	pub fn tick(mut q: Query<(&Self, &mut Transform)>, t: Res<Time>) {
		for (item, mut xform) in &mut q {
			xform.scale += item.scale * t.delta_seconds()
		}
	}
}

#[derive(Component)]
pub struct TargetScale {
	pub scale: Vec3,
}
impl TargetScale {
	pub fn tick(
		mut q: Query<(
			&Self,
			&mut Transform,
			&InitialTransform,
			&TimeCreated,
			&Lifetime,
		)>,
		t: Res<Time>,
	) {
		for (target, mut xform, init_xform, t_created, lifetime) in &mut q {
			xform.scale = init_xform.scale.lerp(
				target.scale,
				t.last_update()
					.unwrap()
					.duration_since(t_created.0)
					.as_secs_f32() / lifetime.0.as_secs_f32(),
			)
		}
	}
}

#[derive(Component)]
pub struct MulTransform {
	pub velocity: Transform,
}
impl MulTransform {
	pub fn tick(mut q: Query<(&Self, &mut Transform)>, t: Res<Time>) {
		let dt = t.delta_seconds();
		for (item, mut xform) in &mut q {
			*xform = *xform
				* Transform {
					translation: xform.translation + item.velocity.translation * dt,
					rotation: xform.rotation.slerp(item.velocity.rotation, dt),
					scale: xform.scale.lerp(item.velocity.scale, dt),
				}
		}
	}
}

#[derive(Component)]
pub struct DynParticleMovement(Box<dyn FnMut(ParticleDataItem, &Time) + Send + Sync + 'static>);

impl DynParticleMovement {
	pub fn tick(mut q: Query<(&mut Self, ParticleData)>, t: Res<Time>) {
		for (item, data) in &mut q {
			(item.map_unchanged(|it| &mut it.0))(data, &*t)
		}
	}
}
