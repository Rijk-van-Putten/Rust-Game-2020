use crate::components::{DamageType, Damageable, Health, Physics, PhysicsLayer, PhysicsType};
use crate::resources::{play_damage_sound, Sounds};
use amethyst::core::Time;
use amethyst::core::Transform;
use amethyst::ecs::{Join, Read, ReadExpect, System, WriteStorage};
use amethyst::{
    assets::AssetStorage,
    audio::{output::Output, Source},
};
use std::ops::Deref;

use crate::vectors::Vector2;

const DRAG: f32 = 10.0;

struct AABB<'a> {
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub do_collision: bool,
    pub damageable: Option<&'a mut Damageable>,
}

impl PartialEq for AABB<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.x1 == other.x1 && self.x2 == other.x2 && self.y1 == other.y1 && self.y2 == other.y2
    }
}

impl AABB<'_> {
    pub fn create_damageable(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        do_collision: bool,
        damageable: &mut Damageable,
    ) -> AABB {
        AABB {
            x1: x - width / 2.0,
            x2: x + width / 2.0,
            y1: y - height / 2.0,
            y2: y + height / 2.0,
            do_collision,
            damageable: Some(damageable),
        }
    }

    pub fn create_normal(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        do_collision: bool,
    ) -> AABB<'static> {
        AABB {
            x1: x - width / 2.0,
            x2: x + width / 2.0,
            y1: y - height / 2.0,
            y2: y + height / 2.0,
            do_collision,
            damageable: None,
        }
    }

    pub fn get_points(&self) -> Vec<Vector2> {
        let mut points = Vec::new();
        points.push(Vector2 {
            x: self.x1,
            y: self.y1,
        });
        points.push(Vector2 {
            x: self.x2,
            y: self.y1,
        });
        points.push(Vector2 {
            x: self.x1,
            y: self.y2,
        });
        points.push(Vector2 {
            x: self.x2,
            y: self.y2,
        });
        return points;
    }
}

struct HitInfo {
    pub damage: f32,
    pub target_id: u16,
}

pub struct PhysicsSystem;

impl<'s> System<'s> for PhysicsSystem {
    type SystemData = (
        WriteStorage<'s, Physics>,
        WriteStorage<'s, Transform>,
        WriteStorage<'s, Damageable>,
        WriteStorage<'s, Health>,
        Read<'s, AssetStorage<Source>>,
        ReadExpect<'s, Sounds>,
        Option<Read<'s, Output>>,
        Read<'s, Time>,
    );

    fn run(
        &mut self,
        (
            mut physics,
            mut transforms,
            mut damageables,
            mut healths,
            asset_storage,
            sounds,
            audio_output,
            time,
        ): Self::SystemData,
    ) {
        const SCALE_MULTIPLIER: f32 = 50.0;
        let mut colliders = Vec::<AABB>::new();

        // Add colliders that are NOT damageble
        for (transf, phys, _) in (&transforms, &physics, !&damageables).join() {
            colliders.push(AABB::create_normal(
                transf.translation().x,
                transf.translation().y,
                transf.scale().x * SCALE_MULTIPLIER,
                transf.scale().y * SCALE_MULTIPLIER,
                PhysicsLayer::collidable(phys.layer),
            ));
        }

        // Add collider that ARE damageable
        for (transf, phys, damageable) in (&transforms, &physics, &mut damageables).join() {
            let aabb = AABB::create_damageable(
                transf.translation().x,
                transf.translation().y,
                transf.scale().x * SCALE_MULTIPLIER,
                transf.scale().y * SCALE_MULTIPLIER,
                PhysicsLayer::collidable(phys.layer),
                damageable,
            );
            colliders.push(aabb);
        }

        let mut hits = Vec::<HitInfo>::new();

        for (phys, transf) in (&mut physics, &mut transforms).join() {
            match phys.physics_type {
                PhysicsType::Static => {}
                PhysicsType::Dynamic => {
                    let collider1 = AABB::create_normal(
                        transf.translation().x,
                        transf.translation().y,
                        transf.scale().x * SCALE_MULTIPLIER,
                        transf.scale().y * SCALE_MULTIPLIER,
                        PhysicsLayer::collidable(phys.layer),
                    );
                    let mut did_collide = false;
                    for collider2 in colliders.iter_mut() {
                        if collider1 == *collider2 {
                            continue;
                        }
                        for point in collider2.get_points().iter() {
                            if point.x >= collider1.x1
                                && point.x <= collider1.x2
                                && point.y >= collider1.y1
                                && point.y <= collider1.y2
                            {
                                did_collide = collider1.do_collision && collider2.do_collision;
                                match &mut collider2.damageable {
                                    Some(d) => {
                                        // TEMP FIX (REALLY BAD CODE)
                                        // Make sure that enemies don't damage other enemies
                                        // ONLY DAMAGE IF phys.id = 1 AND damageable.damage_type = Player
                                        // OR phys.id > 1 AND damageable.damage_type = Enemy
                                        if phys.id == 1 && d.damage_type == DamageType::Player
                                            || phys.id > 1 && d.damage_type == DamageType::Enemy
                                        {
                                            d.destroyed = true;
                                            hits.push(HitInfo {
                                                damage: d.damage,
                                                target_id: phys.id,
                                            });
                                        }
                                    }
                                    None => {}
                                }
                            }
                        }
                    }
                    if did_collide {
                        phys.velocity = phys.velocity * 0.01;
                    }
                    if phys.drag {
                        phys.velocity = phys.velocity * (1.0 - time.delta_seconds() * DRAG);
                    }
                    let old_translation = transf.translation().clone();
                    transf.set_translation(
                        old_translation + (phys.velocity * time.delta_seconds()).to_vector3(),
                    );
                }
            }
        }

        if hits.len() == 0 {
            return;
        }

        for (phys, health) in (&mut physics, &mut healths).join() {
            for hit in &hits {
                if hit.target_id == phys.id {
                    health.hp -= hit.damage;
                    play_damage_sound(
                        &*sounds,
                        &asset_storage,
                        audio_output.as_ref().map(|o| o.deref()),
                    );
                }
            }
        }
    }
}
