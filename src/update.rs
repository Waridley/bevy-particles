use bevy::prelude::*;

use super::*;

#[derive(Debug, Component, Reflect)]
pub struct Linear {
	pub velocity: Vec3,
}
impl Linear {
	pub fn tick(
		mut q: Query<(&Self, &mut Transform, &InitialTransform, &TimeCreated)>,
		t: Res<Time<Real>>,
	) {
		q.par_iter_mut()
			.for_each(|(item, mut xform, init_xform, t_created)| {
				xform.translation = init_xform.translation
					+ (item.velocity
						* t.last_update()
							.unwrap()
							.duration_since(t_created.0)
							.as_secs_f32());
			});
	}
}

#[derive(Debug, Component, Reflect)]
pub struct Angular {
	pub velocity: Quat,
}
impl Angular {
	pub fn tick(mut q: Query<(&Self, &mut Transform)>, t: Res<Time>) {
		q.par_iter_mut().for_each(|(item, mut xform)| {
			xform.rotation = xform.rotation.slerp(item.velocity, t.delta_seconds())
		});
	}
}

#[derive(Debug, Component, Reflect)]
pub struct MulScale {
	pub scale: Vec3,
}
impl MulScale {
	pub fn tick(mut q: Query<(&Self, &mut Transform)>, t: Res<Time>) {
		q.par_iter_mut().for_each(|(item, mut xform)| {
			xform.scale *= Vec3::ONE.lerp(item.scale, t.delta_seconds())
		});
	}
}

#[derive(Debug, Component, Reflect)]
pub struct AddScale {
	pub scale: Vec3,
}
impl AddScale {
	pub fn tick(mut q: Query<(&Self, &mut Transform)>, t: Res<Time>) {
		q.par_iter_mut()
			.for_each(|(item, mut xform)| xform.scale += item.scale * t.delta_seconds());
	}
}

#[derive(Debug, Component, Reflect)]
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
		t: Res<Time<Real>>,
	) {
		q.par_iter_mut()
			.for_each(|(target, mut xform, init_xform, t_created, lifetime)| {
				xform.scale = init_xform.scale.lerp(
					target.scale,
					t.last_update()
						.unwrap()
						.duration_since(t_created.0)
						.as_secs_f32() / lifetime.0.as_secs_f32(),
				)
			});
	}
}

#[derive(Debug, Component, Reflect)]
pub struct TargetTransform {
	pub final_xform: Transform,
}
impl TargetTransform {
	pub fn tick(
		mut q: Query<(
			&Self,
			&mut Transform,
			&InitialTransform,
			&TimeCreated,
			&Lifetime,
		)>,
		t: Res<Time<Real>>,
	) {
		q.par_iter_mut()
			.for_each(|(item, mut xform, init_xform, init_t, lifetime)| {
				let elapsed = t.last_update().unwrap().duration_since(**init_t);
				let s = elapsed.as_secs_f32() / lifetime.as_secs_f32();
				*xform = Transform {
					translation: init_xform.translation.lerp(item.final_xform.translation, s),
					rotation: init_xform.rotation.slerp(item.final_xform.rotation, s),
					scale: init_xform.scale.lerp(item.final_xform.scale, s),
				}
			});
	}
}

pub trait ParticleUpdateFn: FnMut(ParticleDataItem, &Time) + Send + Sync + 'static {}
impl<F> ParticleUpdateFn for F where F: FnMut(ParticleDataItem, &Time) + Send + Sync + 'static {}

#[derive(Component)]
pub struct DynParticleUpdate(Box<dyn ParticleUpdateFn>);

impl DynParticleUpdate {
	pub fn tick(mut q: Query<(&mut Self, ParticleData)>, t: Res<Time>) {
		q.par_iter_mut()
			.for_each(|(item, data)| (item.map_unchanged(|it| &mut it.0))(data, &t));
	}
}
