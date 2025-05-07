use crate::bullet::RustBullet;
use crate::{BULLET_DAMAGE, BULLET_DISTANCE, BULLET_REPEL, MAX_AMMO, MAX_BULLET_HIT, RELOAD_TIME};
use godot::builtin::{Vector2, real};
use godot::classes::{AudioStreamPlayer2D, GpuParticles2D, INode2D, Node2D, PackedScene};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::ops::Add;
use std::time::{Duration, Instant};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWeapon {
    #[export]
    damage: i64,
    #[export]
    distance: real,
    #[export]
    clip: i64,
    #[export]
    repel: real,
    #[export]
    max_hit_count: u8,
    #[export]
    fire_cooldown: u32,
    #[export]
    reload_time: u32,
    ammo: i64,
    last_shot_time: Instant,
    bullet_scene: OnReady<Gd<PackedScene>>,
    bullet_point: OnReady<Gd<Node2D>>,
    fire_flash: OnReady<Gd<GpuParticles2D>>,
    fire_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    reload_start_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    reload_end_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWeapon {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            damage: BULLET_DAMAGE,
            distance: BULLET_DISTANCE,
            clip: MAX_AMMO,
            repel: BULLET_REPEL,
            max_hit_count: MAX_BULLET_HIT,
            fire_cooldown: 100,
            reload_time: RELOAD_TIME,
            ammo: MAX_AMMO,
            last_shot_time: Instant::now(),
            bullet_scene: OnReady::from_loaded("res://scenes/rust_bullet.tscn"),
            bullet_point: OnReady::from_node("BulletPoint"),
            fire_flash: OnReady::from_node("GpuParticles2D"),
            fire_audio: OnReady::from_node("FireAudio"),
            reload_start_audio: OnReady::from_node("ReloadStartAudio"),
            reload_end_audio: OnReady::from_node("ReloadEndAudio"),
            base,
        }
    }

    fn ready(&mut self) {
        self.last_shot_time -= Duration::from_millis(self.fire_cooldown as u64);
    }
}

#[godot_api]
impl RustWeapon {
    pub fn fire(
        &mut self,
        player_damage: i64,
        player_distance: real,
        player_max_hit_count: u8,
        player_repel: real,
    ) {
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
                .direction_to(self.get_mouse_position())
                .normalized();
            bullet.set_global_position(bullet_point);
            let mut gd_mut = bullet.bind_mut();
            gd_mut.set_bullet_point(bullet_point);
            gd_mut.set_final_distance(player_distance.add(self.distance));
            gd_mut.set_final_damage(player_damage.saturating_add(self.damage));
            gd_mut.set_final_max_hit_count(player_max_hit_count.saturating_add(self.max_hit_count));
            gd_mut.set_final_repel(player_repel.add(self.repel));
            gd_mut.set_direction(direction);
            drop(gd_mut);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&bullet);
                    self.fire_flash.restart();
                    self.fire_audio.play();
                    self.last_shot_time = now;
                    self.ammo -= 1;
                }
            }
        }
    }

    pub fn reload(&mut self) {
        if self.clip == self.ammo {
            return;
        }
        self.reload_start_audio.play();
    }

    pub fn reloaded(&mut self) -> i64 {
        if self.clip == self.ammo {
            return self.clip;
        }
        self.reload_end_audio.play();
        self.ammo = self.clip;
        self.ammo
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        self.base().get_canvas_transform().affine_inverse()
            * self
                .base()
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }

    pub fn must_reload(&self) -> bool {
        0 == self.ammo
    }

    pub fn get_ammo(&self) -> i64 {
        self.ammo
    }
}
