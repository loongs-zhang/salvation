use super::*;
use crate::SAVE;
use godot::builtin::StringName;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;

impl Serialize for RustWeapon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RustWeapon", 1)?;
        state.serialize_field("name", &self.base().get_name())?;
        state.serialize_field("silenced", &self.silenced)?;
        state.serialize_field("damage", &self.damage)?;
        state.serialize_field("weight", &self.weight)?;
        state.serialize_field("distance", &self.distance)?;
        state.serialize_field("clip", &self.clip)?;
        state.serialize_field("explode", &self.explode)?;
        state.serialize_field("pull_after_reload", &self.pull_after_reload)?;
        state.serialize_field("repel", &self.repel)?;
        state.serialize_field("penetrate", &self.penetrate)?;
        state.serialize_field("fire_cooldown", &self.fire_cooldown)?;
        state.serialize_field("reload_time", &self.reload_time)?;
        state.serialize_field("reload_part", &self.reload_part)?;
        state.serialize_field("reloading", &self.reloading)?;
        state.serialize_field("ammo", &self.ammo)?;
        state.end()
    }
}

#[derive(Debug, Deserialize)]
struct WeaponData {
    name: StringName,
    silenced: bool,
    damage: i64,
    weight: real,
    distance: real,
    clip: i32,
    explode: bool,
    pull_after_reload: bool,
    repel: real,
    penetrate: real,
    fire_cooldown: real,
    reload_time: real,
    reload_part: bool,
    reloading: real,
    ammo: i32,
}

#[godot_api(secondary)]
impl RustWeapon {
    #[func]
    pub fn on_save(&self) {
        let name = self.base().get_class().to_string();
        let data = serde_json::to_string(&self).unwrap();
        if let Some(mut vec) = SAVE.get_mut(&name) {
            vec.insert(data);
        } else {
            SAVE.insert(name, HashSet::from([data]));
        }
    }

    #[func]
    pub fn on_load(&mut self) {
        let name = self.base().get_class().to_string();
        if let Some((_, vec)) = SAVE.remove(&name) {
            for json in &vec {
                if let Ok(save_data) = serde_json::from_str::<WeaponData>(json) {
                    if self.base().get_name() != save_data.name {
                        continue;
                    }
                    self.silenced = save_data.silenced;
                    self.damage = save_data.damage;
                    self.weight = save_data.weight;
                    self.distance = save_data.distance;
                    self.clip = save_data.clip;
                    self.explode = save_data.explode;
                    self.pull_after_reload = save_data.pull_after_reload;
                    self.repel = save_data.repel;
                    self.penetrate = save_data.penetrate;
                    self.fire_cooldown = save_data.fire_cooldown;
                    self.reload_time = save_data.reload_time;
                    self.reload_part = save_data.reload_part;
                    self.reloading = save_data.reloading;
                    self.ammo = save_data.ammo;
                    self.update_ammo_hud();
                }
            }
        }
    }
}
