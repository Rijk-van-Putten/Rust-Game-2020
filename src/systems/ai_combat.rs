use crate::components::{Damageable, Enemy, EnemyType, Health, Lifetime, Physics, Player, PhysicsLayer, PhysicsType};
use crate::resources::GameInfo;
use crate::resources::{play_fire_sound, Sounds};
use crate::vectors::Vector2;
use amethyst::core::{Time, Transform};
use amethyst::ecs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use amethyst::renderer::SpriteRender;
use amethyst::{
    assets::AssetStorage,
    audio::{output::Output, Source},
};
use amethyst::core::math::{Point3, Vector3};
use std::ops::Deref;
use crate::resources::SpriteSheetHolder;

pub struct AICombatSystem;

const MELEE_DPS: f32 = 50.0;


const PROJECTILE_SPEED: f32 = 200.0;
const PROJECTILE_SPAWN_OFFSET: f32 = 22.0;
const PROJECTILE_DAMAGE: f32 = 40.0;


impl<'s> System<'s> for AICombatSystem {
    type SystemData = (
        WriteStorage<'s, Enemy>,
        WriteStorage<'s, Transform>,
        ReadStorage<'s, Player>,
        WriteStorage<'s, Health>,
        Read<'s, GameInfo>,
        Read<'s, Time>,
        Entities<'s>,
        WriteStorage<'s, Damageable>,
        WriteStorage<'s, SpriteRender>,
        WriteStorage<'s, Physics>,
        WriteStorage<'s, Lifetime>,
        Read<'s, AssetStorage<Source>>,
        ReadExpect<'s, Sounds>,
        Option<Read<'s, Output>>,
        Read<'s, SpriteSheetHolder>
    );

    fn run(
        &mut self,
        (
            mut enemies,
            mut transforms,
            players,
            mut healths,
            game_info,
            time,
            entities,
            mut damageables,
            mut sprite_renderers,
            mut physics,
            mut lifetimes,
            asset_storage,
            sounds,
            audio_output,
            sprite_sheet_holder,
        ): Self::SystemData,
    ) {
        let mut total_melee_damage = 0.0;
        let sprite_sheet = match &sprite_sheet_holder.sprite_sheet {
            None => return,
            Some(s) => s,
        };
        for (enemy, transform) in (&mut enemies, &transforms).join() {
            
            match enemy.enemy_type {
                EnemyType::Melee => {
                    if enemy.can_attack {
                        total_melee_damage += MELEE_DPS * time.delta_seconds();
                    }
                },
                EnemyType::Range => {
                    // TODO: INSTEAD OF DOUBLE MUTABLE CREATE A LIST OF THE POSITIONS AND DIRECTIONS & CREATE ALL OF THE PROJECTILES LATER
                    let target = game_info.player_position;
                    let curr_pos = Vector2::new(transform.translation().x, transform.translation().y);
                    let direction = (target - curr_pos).normalized();
                    let spawn_position = curr_pos + direction * PROJECTILE_SPAWN_OFFSET;
                    let mut projectile_transform =
                        Transform::from(Vector3::new(spawn_position.x, spawn_position.y, 0.0));

                    projectile_transform.set_rotation_2d(-direction.get_radians());
                    projectile_transform.set_scale(Vector3::new(0.3, 0.3, 1.0));
                    entities
                        .build_entity()
                        .with(projectile_transform, &mut transforms)
                        .with(
                            Damageable {
                                damage: PROJECTILE_DAMAGE,
                                destroyed: false,
                            },
                            &mut damageables,
                        )
                        .with(
                            SpriteRender {
                                sprite_sheet: sprite_sheet.clone(),
                                sprite_number: 2,
                            },
                            &mut sprite_renderers,
                        )
                        .with(
                            Physics::simple(
                                PhysicsType::Dynamic,
                                PhysicsLayer::Projectile,
                                direction * PROJECTILE_SPEED,
                            ),
                            &mut physics,
                        )
                        .with(Lifetime { lifetime: 5.0 }, &mut lifetimes)
                        .build();
                    play_fire_sound(
                        &*sounds,
                        &asset_storage,
                        audio_output.as_ref().map(|o| o.deref()),
                    );
                },
            };
        }
        for (_player, health) in (&players, &mut healths).join() {
            health.hp -= total_melee_damage;
        }
        // TODO: Deal the player total_melee_damage amount of damage
    }
}
