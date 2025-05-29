use crate::{SAVE, SAVE_PATH};
use dashmap::DashMap;
use godot::classes::file_access::ModeFlags;
use godot::classes::{Engine, FileAccess, INode, Node, Node2D, SceneTree};
use godot::obj::{Base, Gd, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::collections::HashSet;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct RustSaveLoader {
    base: Base<Node>,
}

#[godot_api]
impl INode for RustSaveLoader {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl RustSaveLoader {
    #[func]
    pub fn load_game(&self) {
        if let Some(file) = FileAccess::open(SAVE_PATH, ModeFlags::READ) {
            let data = file.get_as_text().to_string();
            if let Ok(save_data) = serde_json::from_str::<DashMap<String, HashSet<String>>>(&data) {
                for (k, v) in save_data {
                    SAVE.insert(k, v);
                }
                if SAVE.is_empty() {
                    return;
                }
                let array = self
                    .base()
                    .get_tree()
                    .unwrap()
                    .get_nodes_in_group("preservable");
                for mut node in array.iter_shared() {
                    if !node.has_method("before_load") {
                        continue;
                    }
                    node.call("before_load", &[]);
                }
                for mut node in array.iter_shared() {
                    if !node.has_method("on_load") {
                        continue;
                    }
                    node.call("on_load", &[]);
                }
            }
        }
    }

    #[func]
    pub fn save_game(&self) {
        for mut node in self
            .base()
            .get_tree()
            .unwrap()
            .get_nodes_in_group("preservable")
            .iter_shared()
        {
            if !node.has_method("on_save") {
                continue;
            }
            node.call("on_save", &[]);
        }
        if let Some(mut file) = FileAccess::open(SAVE_PATH, ModeFlags::WRITE) {
            file.store_string(&serde_json::to_string_pretty(&*SAVE).unwrap());
        }
    }

    pub fn get() -> Gd<Self> {
        Engine::singleton()
            .get_main_loop()
            .unwrap()
            .cast::<SceneTree>()
            .get_root()
            .unwrap()
            .get_node_as::<Node2D>("RustWorld")
            .get_node_as::<Self>("RustSaveLoader")
    }
}
