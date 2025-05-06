use godot::classes::{INode2D, Label, Node2D};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::ToGodot;
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct ZombieHit {
    hit_value: OnReady<Gd<Label>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ZombieHit {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            hit_value: OnReady::from_node("HitValue"),
            base,
        }
    }
}

#[godot_api]
impl ZombieHit {
    pub fn show_hit_value(&mut self, value: i64) {
        let position = self.base().get_global_position();
        self.hit_value.set_text(&format!("-{}", value));
        self.hit_value.show();
        let mut tween = self
            .base_mut()
            .create_tween()
            .expect("Failed to create tween");
        tween.tween_property(
            &self.base.to_gd(),
            "position:y",
            &(position.y - 25.0).to_variant(),
            0.5,
        );
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
