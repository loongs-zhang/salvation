use super::*;
use crate::SAVE;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;

impl Serialize for RustPlayer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RustPlayer", 1)?;
        state.serialize_field("name", &self.base().get_name())?;
        state.serialize_field("global_position", &self.base().get_global_position())?;
        state.serialize_field("player_name", &self.player_name)?;
        state.serialize_field("invincible", &self.invincible)?;
        state.serialize_field("current_weapon_index", &self.current_weapon_index)?;
        state.serialize_field("lives", &self.lives)?;
        state.serialize_field("damage", &self.damage)?;
        state.serialize_field("distance", &self.distance)?;
        state.serialize_field("penetrate", &self.penetrate)?;
        state.serialize_field("repel", &self.repel)?;
        state.serialize_field("health", &self.health)?;
        state.serialize_field("speed", &self.speed)?;
        state.serialize_field("level_up_barrier", &self.level_up_barrier)?;
        state.serialize_field("grenade_cooldown", &self.grenade_cooldown)?;
        state.serialize_field("chop_cooldown", &self.chop_cooldown)?;
        state.serialize_field("current_level_up_barrier", &self.current_level_up_barrier)?;
        state.serialize_field("current_lives", &self.current_lives)?;
        state.serialize_field("current_health", &self.current_health)?;
        state.serialize_field("impact_position", &self.impact_position)?;
        state.serialize_field("left_impact_time", &self.left_impact_time)?;
        state.serialize_field("score", &self.score)?;
        state.serialize_field("died", &self.died)?;
        state.serialize_field("kill_count", &self.kill_count)?;
        state.serialize_field("kill_boss_count", &self.kill_boss_count)?;
        state.end()
    }
}

#[derive(Debug, Deserialize)]
struct PlayerData {
    global_position: Vector2,
    player_name: GString,
    invincible: bool,
    current_weapon_index: i32,
    lives: u32,
    damage: i64,
    distance: real,
    penetrate: real,
    repel: real,
    health: u32,
    speed: real,
    level_up_barrier: u32,
    grenade_cooldown: real,
    chop_cooldown: real,
    current_level_up_barrier: u32,
    current_lives: u32,
    current_health: u32,
    impact_position: Vector2,
    left_impact_time: f64,
    score: u32,
    died: u32,
    kill_count: u32,
    kill_boss_count: u32,
}

#[godot_api(secondary)]
impl RustPlayer {
    #[func]
    pub fn on_save(&self) {
        let name = self.base().get_class().to_string();
        let data = serde_json::to_string(&self).unwrap();
        SAVE.insert(name, HashSet::from([data]));
    }

    #[func]
    pub fn on_load(&mut self) {
        let name = self.base().get_class().to_string();
        if let Some((_, vec)) = SAVE.remove(&name) {
            let json = vec.iter().next().unwrap();
            if let Ok(save_data) = serde_json::from_str::<PlayerData>(json) {
                self.base_mut()
                    .set_global_position(save_data.global_position);
                self.player_name = save_data.player_name;
                self.invincible = save_data.invincible;
                self.current_weapon_index = save_data.current_weapon_index;
                self.lives = save_data.lives;
                self.damage = save_data.damage;
                self.distance = save_data.distance;
                self.penetrate = save_data.penetrate;
                self.repel = save_data.repel;
                self.health = save_data.health;
                self.speed = save_data.speed;
                self.level_up_barrier = save_data.level_up_barrier;
                self.grenade_cooldown = save_data.grenade_cooldown;
                self.chop_cooldown = save_data.chop_cooldown;
                self.current_level_up_barrier = save_data.current_level_up_barrier;
                self.current_lives = save_data.current_lives;
                self.current_health = save_data.current_health;
                self.impact_position = save_data.impact_position;
                self.left_impact_time = save_data.left_impact_time;
                self.score = save_data.score;
                self.died = save_data.died;
                self.kill_count = save_data.kill_count;
                self.kill_boss_count = save_data.kill_boss_count;
                self.ready();
            }
        }
    }
}
