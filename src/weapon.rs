use crate::bullet::RustBullet;
use crate::{BULLET_DAMAGE, MAX_AMMO, MAX_BULLET_HIT};
use godot::builtin::Vector2;
use godot::classes::{INode2D, Node2D, PackedScene};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::{GodotClass, godot_api};
use std::time::{Duration, Instant};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWeapon {
    #[export]
    damage: i64,
    #[export]
    ammo: i64,
    #[export]
    max_hit_count: u8,
    #[export]
    fire_cooldown: u32,
    last_shot_time: Instant,
    bullet_scene: OnReady<Gd<PackedScene>>,
    bullet_point: OnReady<Gd<Node2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWeapon {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            damage: BULLET_DAMAGE,
            ammo: MAX_AMMO,
            max_hit_count: MAX_BULLET_HIT,
            fire_cooldown: 200,
            last_shot_time: Instant::now(),
            bullet_scene: OnReady::from_loaded("res://scenes/rust_bullet.tscn"),
            bullet_point: OnReady::from_node("BulletPoint"),
            base,
        }
    }

    fn ready(&mut self) {
        self.last_shot_time -= Duration::from_millis(self.fire_cooldown as u64);
    }
}

#[godot_api]
impl RustWeapon {
    pub fn fire(&mut self, player_damage: i64, player_max_hit_count: u8) {
        if 0 == self.ammo {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.last_shot_time)
            < Duration::from_millis(self.fire_cooldown as u64)
        {
            return;
        }
        if let Some(mut bullet) = self.bullet_scene.try_instantiate_as::<RustBullet>() {
            let bullet_point = self.bullet_point.get_global_position();
            let direction = self
                .base()
                .get_global_position()
                .direction_to(self.get_mouse_position());
            bullet.set_global_position(bullet_point);
            let mut gd_mut = bullet.bind_mut();
            gd_mut.set_final_damage(player_damage.saturating_add(self.damage));
            gd_mut.set_final_max_hit_count(player_max_hit_count.saturating_add(self.max_hit_count));
            gd_mut.set_direction(direction);
            drop(gd_mut);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&bullet);
                    self.last_shot_time = now;
                    self.ammo -= 1;
                }
            }
        }
    }

    pub fn reload(&mut self) {
        if MAX_AMMO == self.ammo {
            return;
        }
        self.ammo = MAX_AMMO;
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
