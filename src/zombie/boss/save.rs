use super::*;
use crate::SAVE;
use crate::level::generator::ZombieGenerator;
use godot::classes::PackedScene;
use godot::tools::load;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;
use std::sync::OnceLock;

#[allow(clippy::declare_interior_mutable_const)]
const SELF: OnceLock<Gd<PackedScene>> = OnceLock::new();

impl Serialize for RustBoss {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RustBoss", 1)?;
        state.serialize_field("name", &self.base().get_name())?;
        state.serialize_field("global_position", &self.base().get_global_position())?;
        state.serialize_field(
            "global_rotation_degrees",
            &self.base().get_global_rotation_degrees(),
        )?;
        state.serialize_field("boss_name", &self.boss_name)?;
        state.serialize_field("invincible", &self.invincible)?;
        state.serialize_field("moveable", &self.moveable)?;
        state.serialize_field("attackable", &self.attackable)?;
        state.serialize_field("collidable", &self.collidable)?;
        state.serialize_field("health", &self.health)?;
        state.serialize_field("speed", &self.speed)?;
        state.end()
    }
}

#[derive(Debug, Deserialize)]
struct BossData {
    global_position: Vector2,
    global_rotation_degrees: f32,
    boss_name: GString,
    invincible: bool,
    moveable: bool,
    attackable: bool,
    collidable: bool,
    health: u32,
    speed: real,
}

#[godot_api(secondary)]
impl RustBoss {
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
            self.clean_body()
        }
    }

    #[func]
    pub fn on_load(&mut self) {
        let name = self.base().get_class().to_string();
        if let Some((_, vec)) = SAVE.remove(&name) {
            if let Some(mut parent) = self.base().get_parent() {
                for json in &vec {
                    if let Ok(save_data) = serde_json::from_str::<BossData>(json) {
                        #[allow(clippy::borrow_interior_mutable_const)]
                        if let Some(mut boss) = SELF
                            .get_or_init(|| load(&self.base().get_scene_file_path()))
                            .try_instantiate_as::<Self>()
                        {
                            boss.set_global_position(save_data.global_position);
                            boss.set_global_rotation_degrees(save_data.global_rotation_degrees);
                            boss.bind_mut().boss_name = save_data.boss_name;
                            boss.bind_mut().invincible = save_data.invincible;
                            boss.bind_mut().moveable = save_data.moveable;
                            boss.bind_mut().attackable = save_data.attackable;
                            boss.bind_mut().collidable = save_data.collidable;
                            boss.bind_mut().health = save_data.health;
                            boss.bind_mut().speed = save_data.speed;
                            parent.add_child(&boss);
                            if let Some(level) = RustLevel::get() {
                                if let Some(generator) =
                                    level.try_get_node_as::<ZombieGenerator>("BossGenerator")
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
