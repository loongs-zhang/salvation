use crate::bullet::RustBullet;
use godot::builtin::Vector2;
use godot::classes::{INode2D, Node2D, PackedScene};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::{GodotClass, godot_api};
use std::time::{Duration, Instant};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWeapon {
    last_shot_time: Instant,
    fire_cooldown: Duration,
    bullet_scene: OnReady<Gd<PackedScene>>,
    bullet_point: OnReady<Gd<Node2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWeapon {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            last_shot_time: Instant::now(),
            fire_cooldown: Duration::from_millis(200),
            bullet_scene: OnReady::from_loaded("res://scenes/rust_bullet.tscn"),
            bullet_point: OnReady::from_node("BulletPoint"),
            base,
        }
    }
}

#[godot_api]
impl RustWeapon {
    pub fn fire(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_shot_time) < self.fire_cooldown {
            return;
        }
        if let Some(mut bullet) = self.bullet_scene.try_instantiate_as::<RustBullet>() {
            let bullet_point = self.bullet_point.get_global_position();
            let direction = self
                .base()
                .get_global_position()
                .direction_to(self.get_mouse_position());
            bullet.set_global_position(bullet_point);
            bullet.bind_mut().set_direction(direction);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&bullet);
                    self.last_shot_time = now;
                }
            }
        }
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        self.base().get_canvas_transform().affine_inverse()
            * self
                .base()
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }
}
