use crate::bullet::RustBullet;
use crate::grenade::RustGrenade;
use crate::player::RustPlayer;
use crate::weapon::hud::WeaponHUD;
use crate::{
    BULLET_DAMAGE, BULLET_DISTANCE, BULLET_PENETRATE, BULLET_REPEL, MAX_AMMO, RELOAD_TIME,
    WEAPON_FIRE_COOLDOWN, random_direction,
};
use godot::builtin::{Array, Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, Control, GpuParticles2D, INode2D, Node, Node2D, Object, PackedScene,
};
use godot::global::godot_error;
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use godot::tools::load;

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
    clip: i32,
    //是否每次都从所有子弹点射出子弹
    #[export]
    explode: bool,
    //子弹类型
    #[export]
    bullet_scenes: Array<Gd<PackedScene>>,
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
    #[export]
    reload_part: bool,
    reloading: real,
    ammo: i32,
    current_fire_cooldown: real,
    current_flash_cooldown: f64,
    hud: OnReady<Gd<WeaponHUD>>,
    bullet_points: OnReady<Gd<Control>>,
    fire_flash: OnReady<Gd<GpuParticles2D>>,
    fire_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    clip_out_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    clip_part_in_audio: OnReady<Gd<AudioStreamPlayer2D>>,
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
            explode: false,
            pull_after_reload: false,
            repel: BULLET_REPEL,
            penetrate: BULLET_PENETRATE,
            fire_cooldown: WEAPON_FIRE_COOLDOWN,
            reload_time: RELOAD_TIME,
            reload_part: false,
            reloading: 0.0,
            ammo: MAX_AMMO,
            current_fire_cooldown: WEAPON_FIRE_COOLDOWN,
            current_flash_cooldown: 0.0,
            hud: OnReady::from_node("WeaponHUD"),
            bullet_scenes: Array::new(),
            bullet_points: OnReady::from_node("BulletPoints"),
            fire_flash: OnReady::from_node("GpuParticles2D"),
            fire_audio: OnReady::from_node("FireAudio"),
            clip_out_audio: OnReady::from_node("ClipOutAudio"),
            clip_part_in_audio: OnReady::from_node("ClipPartInAudio"),
            clip_in_audio: OnReady::from_node("ClipInAudio"),
            bolt_pull_audio: OnReady::from_node("BoltPullAudio"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        self.current_fire_cooldown -= delta as real;
        self.current_flash_cooldown -= delta;
        if self.ammo < self.clip && self.reloading > 0.0 {
            self.reloading -= delta as real;
        } else if self.reloading < 0.0 && !self.clip_in_audio.is_playing() {
            self.reloading = 0.0;
            self.clip_in_audio.play();
        }
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
        self.clip_part_in_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::on_clip_part_in_finished);
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
        if self.bullet_scenes.is_empty() {
            self.bullet_scenes
                .push(&load::<PackedScene>("res://scenes/rust_bullet.tscn"));
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
        let vec: Vec<Gd<PackedScene>> = self.bullet_scenes.iter_shared().collect();
        for bullet_scene in vec {
            let r = if self.explode {
                //散弹枪一次性从所有子弹点射出子弹
                let mut r = Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to instantiate bullet or grenade",
                ));
                for bullet_point in self.bullet_points.get_children().iter_shared() {
                    //增加子弹散射
                    let direction = self
                        .base()
                        .get_global_position()
                        .direction_to(self.get_mouse_position() + random_direction() * 30.0)
                        .normalized();
                    r = self.do_fire(
                        player_damage,
                        player_distance,
                        player_penetrate,
                        player_repel,
                        &bullet_scene,
                        bullet_point.cast::<Node2D>().get_global_position(),
                        direction,
                    );
                    if r.is_err() {
                        break;
                    }
                }
                r
            } else {
                let direction = self
                    .base()
                    .get_global_position()
                    .direction_to(self.get_mouse_position())
                    .normalized();
                //加特林开火时多枪管轮询选点射击
                let bullet_point = self
                    .bullet_points
                    .get_children()
                    .pick_random()
                    .unwrap()
                    .cast::<Node2D>()
                    .get_global_position();
                self.do_fire(
                    player_damage,
                    player_distance,
                    player_penetrate,
                    player_repel,
                    &bullet_scene,
                    bullet_point,
                    direction,
                )
            };
            if r.is_err() {
                godot_error!("Failed to instantiate bullet or grenade");
                return;
            }
            if self.current_flash_cooldown <= 0.0 {
                self.fire_flash.restart();
                self.current_flash_cooldown = self.fire_flash.get_lifetime() * 0.25;
            }
            self.fire_audio.play();
            self.current_fire_cooldown = self.fire_cooldown;
            self.ammo -= 1;
            self.base()
                .get_node_as::<WeaponHUD>("WeaponHUD")
                .bind_mut()
                .update_ammo_hud(self.ammo, self.clip);
        }
    }

    fn do_fire(
        &mut self,
        player_damage: i64,
        player_distance: real,
        player_penetrate: real,
        player_repel: real,
        bullet_scene: &Gd<PackedScene>,
        bullet_point: Vector2,
        direction: Vector2,
    ) -> std::io::Result<()> {
        if let Some(mut bullet) = bullet_scene.try_instantiate_as::<RustBullet>() {
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
                    return Ok(());
                }
            }
        } else if let Some(mut grenade) = bullet_scene.try_instantiate_as::<RustGrenade>() {
            grenade.set_global_position(bullet_point);
            let mut gd_mut = grenade.bind_mut();
            gd_mut.set_bullet_point(bullet_point);
            gd_mut.set_final_distance(player_distance + self.distance);
            gd_mut.set_final_damage(player_damage.saturating_add(self.damage));
            gd_mut.set_final_repel(player_repel + self.repel);
            gd_mut.throw(direction);
            drop(gd_mut);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&grenade);
                    return Ok(());
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to instantiate bullet or grenade",
        ))
    }

    pub fn reload(&mut self) -> bool {
        if self.is_reloaded()
            || self.reloading > 0.0
            || self.clip_out_audio.is_playing()
            || self.reload_part && self.clip_part_in_audio.is_playing()
            || self.clip_in_audio.is_playing()
            || self.pull_after_reload && self.bolt_pull_audio.is_playing()
        {
            return false;
        }
        self.reloading = self.reload_time
            - self.clip_out_audio.get_stream().unwrap().get_length() as real
            - self.clip_in_audio.get_stream().unwrap().get_length() as real;
        if self.reload_part {
            self.reloading -= self.clip_part_in_audio.get_stream().unwrap().get_length() as real;
        }
        if self.pull_after_reload {
            self.reloading -= self.clip_in_audio.get_stream().unwrap().get_length() as real;
        }
        self.reloading = self.reloading.max(0.0);
        self.clip_out_audio.play();
        true
    }

    #[func]
    pub fn on_clip_out_finished(&mut self) {
        if self.reloading > 0.0
            || self.clip_in_audio.is_playing()
            || self.clip_part_in_audio.is_playing()
        {
            return;
        }
        if self.reload_part {
            self.clip_part_in_audio.play();
        } else {
            self.clip_in_audio.play();
        }
    }

    #[func]
    pub fn on_clip_part_in_finished(&mut self) {
        self.ammo += 1;
        self.ammo = self.ammo.min(self.clip);
        self.update_ammo_hud();
        self.get_rust_player().call_deferred("reloading", &[]);
        if self.ammo == self.clip {
            self.clip_in_audio.play();
            return;
        }
        self.clip_part_in_audio.play();
    }

    #[func]
    pub fn on_clip_in_finished(&mut self) {
        if self.pull_after_reload {
            self.bolt_pull_audio.play();
            return;
        }
        // 无需拉栓
        self.on_bolt_pull_finished();
    }

    #[func]
    pub fn on_bolt_pull_finished(&mut self) {
        self.ammo = self.clip;
        self.update_ammo_hud();
        self.get_rust_player().call_deferred("reloaded", &[]);
    }

    pub fn stop_reload(&mut self) {
        if self.is_reloaded() {
            return;
        }
        self.clip_out_audio.stop();
        self.clip_part_in_audio.stop();
        self.clip_in_audio.stop();
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

    pub fn get_rust_player(&mut self) -> Gd<RustPlayer> {
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                return root
                    .get_node_as::<Node>("RustWorld")
                    .get_node_as::<RustPlayer>("RustPlayer");
            }
        }
        panic!("RustPlayer not found");
    }

    pub fn must_reload(&self) -> bool {
        0 == self.ammo
    }
}
