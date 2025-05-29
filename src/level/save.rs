use super::*;
use crate::SAVE;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::HashSet;

impl Serialize for RustLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RustLevel", 1)?;
        state.serialize_field("hell", &self.hell)?;
        state.serialize_field("level", &self.level)?;
        state.serialize_field("grow_rate", &self.grow_rate)?;
        state.serialize_field("rampage_time", &self.rampage_time)?;
        state.serialize_field("zombie_refresh_time", &self.zombie_refresh_time)?;
        state.serialize_field("boomer_refresh_time", &self.boomer_refresh_time)?;
        state.serialize_field("boss_refresh_time", &self.boss_refresh_time)?;
        state.serialize_field("left_rampage_time", &self.left_rampage_time)?;
        state.serialize_field("zombie_killed", &self.zombie_killed)?;
        state.serialize_field("boss_killed", &self.boss_killed)?;
        state.end()
    }
}

#[godot_api(secondary)]
impl RustLevel {
    #[func]
    pub fn on_save(&self) {
        let name = self.base().get_class().to_string();
        let data = serde_json::to_string(&self).unwrap();
        SAVE.insert(name, HashSet::from([data]));
    }

    // todo on_load生成僵尸，再调用僵尸自己的on_load调整
}
