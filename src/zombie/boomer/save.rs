use super::*;
use crate::level::generator::ZombieGenerator;
use godot::classes::PackedScene;
use godot::tools::load;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;
use std::sync::OnceLock;

#[allow(clippy::declare_interior_mutable_const)]
const SELF: OnceLock<Gd<PackedScene>> = OnceLock::new();

impl Serialize for RustBoomer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RustBoomer", 1)?;
        state.serialize_field("name", &self.base().get_name())?;
        state.serialize_field("global_position", &self.base().get_global_position())?;
        state.serialize_field(
            "global_rotation_degrees",
            &self.base().get_global_rotation_degrees(),
        )?;
        state.serialize_field("boomer_name", &self.boomer_name)?;
        state.serialize_field("invincible", &self.invincible)?;
        state.serialize_field("moveable", &self.moveable)?;
        state.serialize_field("rotatable", &self.rotatable)?;
        state.serialize_field("detonable", &self.detonable)?;
        state.serialize_field("detonate_countdown", &self.detonate_countdown)?;
        state.serialize_field("health", &self.health)?;
        state.serialize_field("speed", &self.speed)?;
        state.serialize_field("rampage_time", &self.rampage_time)?;
        state.serialize_field("alarm_time", &self.alarm_time)?;
        state.serialize_field("current_alarm_time", &self.current_alarm_time)?;
        state.serialize_field("pursuit_direction", &self.pursuit_direction)?;
        state.end()
    }
}

#[derive(Debug, Deserialize)]
struct BoomerData {
    global_position: Vector2,
    global_rotation_degrees: f32,
    boomer_name: GString,
    invincible: bool,
    moveable: bool,
    rotatable: bool,
    detonable: bool,
    detonate_countdown: f64,
    health: u32,
    speed: real,
    rampage_time: real,
    alarm_time: real,
    current_alarm_time: real,
    pursuit_direction: bool,
}

#[godot_api(secondary)]
impl RustBoomer {
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
            self.clean_body();
        }
    }

    #[func]
    pub fn on_load(&mut self) {
        let name = self.base().get_class().to_string();
        if let Some((_, vec)) = SAVE.remove(&name) {
            if let Some(mut parent) = self.base().get_parent() {
                for json in &vec {
                    if let Ok(save_data) = serde_json::from_str::<BoomerData>(json) {
                        #[allow(clippy::borrow_interior_mutable_const)]
                        if let Some(mut boomer) = SELF
                            .get_or_init(|| load(&self.base().get_scene_file_path()))
                            .try_instantiate_as::<Self>()
                        {
                            boomer.set_global_position(save_data.global_position);
                            boomer.set_global_rotation_degrees(save_data.global_rotation_degrees);
                            boomer.bind_mut().boomer_name = save_data.boomer_name;
                            boomer.bind_mut().invincible = save_data.invincible;
                            boomer.bind_mut().moveable = save_data.moveable;
                            boomer.bind_mut().rotatable = save_data.rotatable;
                            boomer.bind_mut().detonable = save_data.detonable;
                            boomer.bind_mut().detonate_countdown = save_data.detonate_countdown;
                            boomer.bind_mut().health = save_data.health;
                            boomer.bind_mut().speed = save_data.speed;
                            boomer.bind_mut().rampage_time = save_data.rampage_time;
                            boomer.bind_mut().alarm_time = save_data.alarm_time;
                            boomer.bind_mut().current_alarm_time = save_data.current_alarm_time;
                            boomer.bind_mut().pursuit_direction = save_data.pursuit_direction;
                            parent.add_child(&boomer);
                            if let Some(level) = RustLevel::get() {
                                if let Some(generator) =
                                    level.try_get_node_as::<ZombieGenerator>("BoomerGenerator")
                                {
                                    generator.bind().add_current();
                                }
                            }
                        }
                    }
                }
                // 清理自己
                parent.remove_child(&self.to_gd());
                self.base_mut().queue_free();
            }
        }
    }
}
