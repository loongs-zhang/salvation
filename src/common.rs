use crate::PlayerUpgrade;
use godot::classes::{INode2D, Label, Node2D};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::ToGodot;
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustMessage {
    message: OnReady<Gd<Label>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustMessage {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            message: OnReady::from_node("Message"),
            base,
        }
    }
}

#[godot_api]
impl RustMessage {
    pub fn show_level_up(&mut self, value: PlayerUpgrade) {
        self.show_message(&format!("{:?} Upgraded", value))
    }

    pub fn show_message(&mut self, value: &str) {
        let position = self.base().get_global_position();
        self.message.set_text(value);
        self.message.show();
        let mut tween = self
            .base_mut()
            .create_tween()
            .expect("Failed to create tween");
        tween
            .tween_property(
                &self.base.to_gd(),
                "position:y",
                &(position.y - 50.0).to_variant(),
                5.0,
            )
            .expect("tween failed");
        tween
            .parallel()
            .expect("tween parallel failed")
            .tween_property(&self.base.to_gd(), "modulate:a", &0.0_f32.to_variant(), 5.0);
        tween.tween_callback(&self.base().callable("clean"));
    }

    pub fn show_hit_value(&mut self, value: i64) {
        let position = self.base().get_global_position();
        self.message.set_text(&format!("-{}", value));
        self.message.show();
        let mut tween = self
            .base_mut()
            .create_tween()
            .expect("Failed to create tween");
        tween
            .tween_property(
                &self.base.to_gd(),
                "position:y",
                &(position.y - 25.0).to_variant(),
                0.5,
            )
            .expect("tween failed");
        tween.tween_property(
            &self.base.to_gd(),
            "position:y",
            &(position.y - 10.0).to_variant(),
            0.3,
        );
        tween
            .parallel()
            .expect("tween parallel failed")
            .tween_property(&self.base.to_gd(), "modulate:a", &0.0_f32.to_variant(), 0.3);
        tween.tween_callback(&self.base().callable("clean"));
    }

    #[func]
    pub fn clean(&mut self) {
        self.base_mut().queue_free();
    }
}
