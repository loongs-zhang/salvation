use super::*;
use crate::SAVE;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;

impl Serialize for RustLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RustLevel", 1)?;
        state.serialize_field("name", &self.base().get_name())?;
        state.serialize_field("hell", &self.hell)?;
        state.serialize_field("level", &self.level)?;
        state.serialize_field("grow_rate", &self.grow_rate)?;
        state.serialize_field("rampage_time", &self.rampage_time)?;
        state.serialize_field("zombie_refresh_time", &self.zombie_refresh_time)?;
        state.serialize_field("boomer_refresh_time", &self.boomer_refresh_time)?;
        state.serialize_field("pitcher_refresh_time", &self.pitcher_refresh_time)?;
        state.serialize_field("rusher_refresh_time", &self.rusher_refresh_time)?;
        state.serialize_field("boss_refresh_time", &self.boss_refresh_time)?;
        state.serialize_field("left_rampage_time", &self.left_rampage_time)?;
        state.end()
    }
}

#[derive(Debug, Deserialize)]
struct LevelData {
    hell: bool,
    level: u32,
    grow_rate: real,
    rampage_time: real,
    zombie_refresh_time: f64,
    boomer_refresh_time: f64,
    pitcher_refresh_time: f64,
    rusher_refresh_time: f64,
    boss_refresh_time: f64,
    left_rampage_time: real,
}

#[godot_api(secondary)]
impl RustLevel {
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
            if let Ok(save_data) = serde_json::from_str::<LevelData>(json) {
                self.hell = save_data.hell;
                self.level = save_data.level;
                self.grow_rate = save_data.grow_rate;
                self.rampage_time = save_data.rampage_time;
                self.zombie_refresh_time = save_data.zombie_refresh_time;
                self.boomer_refresh_time = save_data.boomer_refresh_time;
                self.pitcher_refresh_time = save_data.pitcher_refresh_time;
                self.rusher_refresh_time = save_data.rusher_refresh_time;
                self.boss_refresh_time = save_data.boss_refresh_time;
                self.left_rampage_time = save_data.left_rampage_time;
            }
            // 生成召唤尸
            for mut child in self.base().get_children().iter_shared() {
                if child.is_class("ZombieGenerator") {
                    child.call("generate_zombie", &[]);
                }
            }
            if let Some(mut tree) = self.base().get_tree() {
                if let Some(mut timer) = tree.create_timer(0.2) {
                    timer.connect("timeout", &self.base().callable("on_summon"));
                }
            }
        }
    }

    #[func]
    pub fn on_summon(&mut self) {
        // 广播，让召唤尸召唤对应的僵尸
        let array = self
            .base()
            .get_parent()
            .unwrap()
            .get_tree()
            .unwrap()
            .get_nodes_in_group("zombie");
        for mut node in array.iter_shared() {
            if !node.has_method("on_load") {
                continue;
            }
            node.call_deferred("on_load", &[]);
        }
    }
}
