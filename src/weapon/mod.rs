use crate::bullet::RustBullet;
use crate::weapon::hud::WeaponHUD;
use crate::{
    BULLET_DAMAGE, BULLET_DISTANCE, BULLET_PENETRATE, BULLET_REPEL, MAX_AMMO, RELOAD_TIME,
    WEAPON_FIRE_COOLDOWN,
};
use godot::builtin::{Vector2, real};
use godot::classes::{AudioStreamPlayer2D, GpuParticles2D, INode2D, Node2D, Object, PackedScene};
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

pub mod hud;

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWeapon {
    //武器伤害
    #[export]
    damage: i64,
    //武器射程
    #[export]
    distance: real,
    //武器弹夹容量
    #[export]
    clip: i64,
    #[export]
    pull_after_reload: bool,
    //武器击退
    #[export]
    repel: real,
    //武器穿透
    #[export]
    penetrate: real,
    #[export]
    fire_cooldown: real,
    #[export]
    reload_time: real,
    //todo 加特林需要增加这个时间，不然换弹太快了
    reloading: real,
    ammo: i64,
    current_fire_cooldown: real,
    hud: OnReady<Gd<WeaponHUD>>,
    bullet_scene: OnReady<Gd<PackedScene>>,
    bullet_point: OnReady<Gd<Node2D>>,
    fire_flash: OnReady<Gd<GpuParticles2D>>,
    fire_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    clip_out_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    clip_in_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    bolt_pull_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWeapon {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            damage: BULLET_DAMAGE,
            distance: BULLET_DISTANCE,
            clip: MAX_AMMO,
            pull_after_reload: false,
            repel: BULLET_REPEL,
            penetrate: BULLET_PENETRATE,
            fire_cooldown: WEAPON_FIRE_COOLDOWN,
            reload_time: RELOAD_TIME,
            reloading: 0.0,
            ammo: MAX_AMMO,
            current_fire_cooldown: WEAPON_FIRE_COOLDOWN,
            hud: OnReady::from_node("WeaponHUD"),
            bullet_scene: OnReady::from_loaded("res://scenes/rust_bullet.tscn"),
            bullet_point: OnReady::from_node("BulletPoint"),
            fire_flash: OnReady::from_node("GpuParticles2D"),
            fire_audio: OnReady::from_node("FireAudio"),
            clip_out_audio: OnReady::from_node("ClipOutAudio"),
            clip_in_audio: OnReady::from_node("ClipInAudio"),
            bolt_pull_audio: OnReady::from_node("BoltPullAudio"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        self.current_fire_cooldown -= delta as real;
    }

    fn ready(&mut self) {
        self.ammo = self.clip;
        self.update_ammo_hud();
        let gd = self.to_gd();
        self.signals()
            .visibility_changed()
            .connect_self(|this: &mut Self| {
                let visible = this.base().is_visible();
                this.hud.set_visible(visible);
            });
        self.clip_out_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::on_clip_out_finished);
        self.clip_in_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::on_clip_in_finished);
        if self.pull_after_reload {
            self.bolt_pull_audio
                .signals()
                .finished()
                .connect_obj(&gd, Self::on_bolt_pull_finished);
        }
    }
}

#[godot_api]
impl RustWeapon {
    #[signal]
    pub fn sig();

    pub fn update_ammo_hud(&mut self) {
        self.hud.bind_mut().update_ammo_hud(self.ammo, self.clip);
    }

    pub fn fire(
        &mut self,
        player_damage: i64,
        player_distance: real,
        player_penetrate: real,
        player_repel: real,
    ) {
        if 0 == self.ammo || self.current_fire_cooldown > 0.0 {
            return;
        }
        //todo 大狙开后后的拉栓音效
        if let Some(mut bullet) = self.bullet_scene.try_instantiate_as::<RustBullet>() {
            //todo 加特林开火时多枪管轮询选点射击
            let bullet_point = self.bullet_point.get_global_position();
            let direction = self
                .base()
                .get_global_position()
                .direction_to(self.get_mouse_position())
                .normalized();
            bullet.set_global_position(bullet_point);
            let mut gd_mut = bullet.bind_mut();
            gd_mut.set_bullet_point(bullet_point);
            gd_mut.set_final_distance(player_distance + self.distance);
            gd_mut.set_final_damage(player_damage.saturating_add(self.damage));
            gd_mut.set_final_penetrate(player_penetrate + self.penetrate);
            gd_mut.set_final_repel(player_repel + self.repel);
            gd_mut.set_direction(direction);
            drop(gd_mut);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&bullet);
                    self.fire_flash.restart();
                    self.fire_audio.play();
                    self.current_fire_cooldown = self.fire_cooldown;
                    self.ammo -= 1;
                    self.update_ammo_hud();
                }
            }
        }
    }

    pub fn reload(&mut self) -> bool {
        if self.is_reloaded() {
            return false;
        }
        self.clip_out_audio.play();
        true
    }

    #[func]
    pub fn on_clip_out_finished(&mut self) {
        //todo 拔出弹夹后等待reload_time s的时间
        self.clip_in_audio.play();
    }

    #[func]
    pub fn on_clip_in_finished(&mut self) {
        if self.pull_after_reload {
            self.bolt_pull_audio.play();
            return;
        }
        self.ammo = self.clip;
        self.update_ammo_hud();
    }

    #[func]
    pub fn on_bolt_pull_finished(&mut self) {
        self.ammo = self.clip;
        self.update_ammo_hud();
    }

    pub fn stop_reload(&mut self) {
        if self.is_reloaded() {
            return;
        }
        self.clip_in_audio.stop();
        self.clip_out_audio.stop();
        self.bolt_pull_audio.stop();
    }

    pub fn is_reloaded(&self) -> bool {
        self.clip == self.ammo
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
