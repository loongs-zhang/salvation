use super::*;
use crate::SAVE;
use godot::builtin::StringName;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;

impl Serialize for ZombieGenerator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ZombieGenerator", 1)?;
        state.serialize_field("name", &self.base().get_name())?;
        state.serialize_field("immediate", &self.immediate)?;
        state.serialize_field("boss", &self.boss)?;
        state.serialize_field("total", &self.total)?;
        state.serialize_field("refresh_count", &self.refresh_count)?;
        state.serialize_field("refresh_barrier", &self.refresh_barrier)?;
        state.serialize_field("refresh_time", &self.refresh_time)?;
        state.serialize_field("max_screen_count", &self.max_screen_count)?;
        state.serialize_field("current_total", &self.current_total)?;
        state.serialize_field("current_refresh_count", &self.current_refresh_count)?;
        state.serialize_field("current", &self.current)?;
        state.serialize_field("current_refresh_barrier", &self.current_refresh_barrier)?;
        state.serialize_field("stopped", &self.timer.is_stopped())?;
        state.end()
    }
}

#[derive(Debug, Deserialize)]
struct GeneratorData {
    name: StringName,
    immediate: bool,
    boss: bool,
    total: u32,
    refresh_count: u32,
    refresh_barrier: u32,
    refresh_time: f64,
    max_screen_count: u32,
    current_total: u32,
    current_refresh_count: u32,
    current: u32,
    current_refresh_barrier: u32,
    stopped: bool,
}

#[godot_api(secondary)]
impl ZombieGenerator {
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
                if let Ok(save_data) = serde_json::from_str::<GeneratorData>(json) {
                    if self.base().get_name() != save_data.name {
                        continue;
                    }
                    self.immediate = save_data.immediate;
                    self.boss = save_data.boss;
                    self.total = save_data.total;
                    self.refresh_count = save_data.refresh_count;
                    self.refresh_barrier = save_data.refresh_barrier;
                    self.refresh_time = save_data.refresh_time;
                    self.max_screen_count = save_data.max_screen_count;
                    self.current_total = save_data.current_total;
                    self.current_refresh_count = save_data.current_refresh_count;
                    self.current = save_data.current;
                    self.current_refresh_barrier = save_data.current_refresh_barrier;
                    if save_data.stopped {
                        self.stop_timer();
                    } else {
                        self.start_timer();
                    }
                }
            }
        }
    }
}
