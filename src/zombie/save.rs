use super::*;
use crate::SAVE;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::HashSet;

impl Serialize for RustZombie {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RustZombie", 1)?;
        state.serialize_field("name", &self.base().get_name())?;
        state.serialize_field("global_position", &self.base().get_global_position())?;
        state.serialize_field(
            "global_rotation_degrees",
            &self.base().get_global_rotation_degrees(),
        )?;
        state.serialize_field("zombie_name", &self.zombie_name)?;
        state.serialize_field("invincible", &self.invincible)?;
        state.serialize_field("moveable", &self.moveable)?;
        state.serialize_field("rotatable", &self.rotatable)?;
        state.serialize_field("attackable", &self.attackable)?;
        state.serialize_field("health", &self.health)?;
        state.serialize_field("speed", &self.speed)?;
        state.serialize_field("rampage_time", &self.rampage_time)?;
        state.serialize_field("alarm_time", &self.alarm_time)?;
        state.serialize_field("current_alarm_time", &self.current_alarm_time)?;
        state.serialize_field("pursuit_direction", &self.pursuit_direction)?;
        state.end()
    }
}

#[godot_api(secondary)]
impl RustZombie {
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
    pub fn before_load(&mut self) {
        if let Some(mut parent) = self.base().get_parent() {
            parent.remove_child(&self.to_gd());
            self.base_mut().queue_free();
        }
    }

    // todo on_load
}
