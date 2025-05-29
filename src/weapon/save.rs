use super::*;
use crate::SAVE;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
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

    // todo on_load
}
