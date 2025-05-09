use crate::PlayerUpgrade;
use godot::classes::{INode2D, Label, Node2D};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::ToGodot;
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct PlayerLevelUp {
    level_up: OnReady<Gd<Label>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for PlayerLevelUp {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            level_up: OnReady::from_node("LevelUp"),
            base,
        }
    }
}

#[godot_api]
impl PlayerLevelUp {
    pub fn show_level_up(&mut self, value: PlayerUpgrade) {
        let position = self.base().get_global_position();
        self.level_up.set_text(&format!("{:?} Upgraded", value));
        self.level_up.show();
        let mut tween = self
            .base_mut()
            .create_tween()
            .expect("Failed to create tween");
        tween.tween_property(
            &self.base.to_gd(),
            "position:y",
            &(position.y - 50.0).to_variant(),
            5.0,
        );
        tween
            .parallel()
            .expect("tween parallel failed")
            .tween_property(&self.base.to_gd(), "modulate:a", &0.0_f32.to_variant(), 5.0);
        tween.tween_callback(&self.base().callable("clean"));
    }

    #[func]
    pub fn clean(&mut self) {
        self.base_mut().queue_free();
    }
}
