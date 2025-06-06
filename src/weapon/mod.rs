use crate::bullet::RustBullet;
use crate::grenade::RustGrenade;
use crate::hud::RustHUD;
use crate::player::RustPlayer;
use crate::{
    BULLET_DAMAGE, BULLET_DISTANCE, BULLET_PENETRATE, BULLET_REPEL, BULLET_SPEED, MAX_AMMO,
    NO_NOISE, RELOAD_TIME, WEAPON_FIRE_COOLDOWN, WeaponState,
};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Array, Callable, Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, Control, GpuParticles2D, INode2D, Node2D, Object, PackedScene, Sprite2D,
};
use godot::global::godot_error;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use godot::tools::load;
use rand::Rng;
use rand::rngs::ThreadRng;
use std::sync::LazyLock;

pub mod save;

static NOISE_POSITION: AtomicCell<Vector2> = AtomicCell::new(NO_NOISE);

#[allow(clippy::declare_interior_mutable_const)]
const BULLET: LazyLock<Gd<PackedScene>> =
    LazyLock::new(|| load("res://scenes/bullets/rust_bullet.tscn"));

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWeapon {
    #[doc = "是否消音"]
    #[export]
    silenced: bool,
    #[doc = "武器伤害"]
    #[export]
    damage: i64,
    #[doc = "持有武器的移动速度比例"]
    #[export]
    weight: real,
    #[doc = "武器射程"]
    #[export]
    distance: real,
    #[doc = "子弹初速"]
    #[export]
    speed: real,
    #[doc = "武器弹夹容量"]
    #[export]
    clip: i32,
    #[doc = "子弹抖动系数"]
    #[export]
    jitter: real,
    #[doc = "是否每次都从所有子弹点射出子弹"]
    #[export]
    explode: bool,
    #[doc = "子弹实体场景"]
    #[export]
    bullet_scenes: Array<Gd<PackedScene>>,
    #[doc = "是否在部署后拉栓"]
    #[export]
    pull_after_deploy: bool,
    #[doc = "是否在重新装填后拉栓"]
    #[export]
    pull_after_reload: bool,
    #[doc = "是否在开火后拉栓"]
    #[export]
    pull_after_fire: bool,
    #[doc = "武器击退系数"]
    #[export]
    repel: real,
    #[doc = "武器穿透系数"]
    #[export]
    penetrate: real,
    #[doc = "武器开火冷却时间"]
    #[export]
    fire_cooldown: real,
    #[doc = "武器重新装填时间"]
    #[export]
    reload_time: real,
    #[doc = "是否支持单发装填"]
    #[export]
    reload_part: bool,
    state: WeaponState,
    reloading: real,
    part_reload_time: real,
    ammo: i32,
    current_fire_cooldown: real,
    current_flash_cooldown: f64,
    current_jitter: real,
    current_jitter_cooldown: real,
    bullet_points: OnReady<Gd<Control>>,
    deploy_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    fire_flash: OnReady<Gd<GpuParticles2D>>,
    fire_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    fire_bolt_pull_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    clip_out_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    clip_part_in_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    clip_in_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    reload_bolt_pull_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWeapon {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            silenced: false,
            damage: BULLET_DAMAGE,
            weight: 1.0,
            distance: BULLET_DISTANCE,
            speed: BULLET_SPEED,
            clip: MAX_AMMO,
            jitter: 0.0,
            explode: false,
            pull_after_deploy: false,
            pull_after_reload: false,
            pull_after_fire: false,
            repel: BULLET_REPEL,
            penetrate: BULLET_PENETRATE,
            fire_cooldown: WEAPON_FIRE_COOLDOWN,
            reload_time: RELOAD_TIME,
            state: WeaponState::Ready,
            reload_part: false,
            reloading: 0.0,
            part_reload_time: 0.0,
            ammo: MAX_AMMO,
            current_fire_cooldown: WEAPON_FIRE_COOLDOWN,
            current_flash_cooldown: 0.0,
            current_jitter: 0.0,
            current_jitter_cooldown: WEAPON_FIRE_COOLDOWN,
            bullet_scenes: Array::new(),
            bullet_points: OnReady::from_node("BulletPoints"),
            deploy_audio: OnReady::from_node("DeployAudio"),
            fire_flash: OnReady::from_node("GpuParticles2D"),
            fire_audio: OnReady::from_node("FireAudio"),
            fire_bolt_pull_audio: OnReady::from_node("FireBoltPullAudio"),
            clip_out_audio: OnReady::from_node("ClipOutAudio"),
            clip_part_in_audio: OnReady::from_node("ClipPartInAudio"),
            clip_in_audio: OnReady::from_node("ClipInAudio"),
            reload_bolt_pull_audio: OnReady::from_node("ReloadBoltPullAudio"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        self.current_fire_cooldown -= delta as real;
        self.current_flash_cooldown -= delta;
        if self.current_jitter > 0.0 && WeaponState::Firing != self.state {
            self.current_jitter_cooldown -= delta as real;
            if self.current_jitter_cooldown <= 0.0 {
                self.current_jitter = (self.current_jitter - self.jitter / 25.0).max(0.0);
                self.current_jitter_cooldown = self.fire_cooldown;
                self.update_jitter_hud();
            }
        }
        if self.ammo >= self.clip || WeaponState::Reloading != self.state {
            return;
        }
        if self.reloading > 0.0 {
            self.reloading -= delta as real;
        } else if self.reloading < 0.0 {
            self.reloading = 0.0;
            if self.reload_part {
                self.on_clip_part_in_finished();
            } else {
                self.on_clip_out_finished();
            }
        }
    }

    fn exit_tree(&mut self) {
        self.bullet_scenes.clear();
    }

    fn ready(&mut self) {
        self.ammo = self.clip;
        self.update_ammo_hud();
        let gd = self.to_gd();
        self.clip_out_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::on_clip_out_finished);
        self.clip_in_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::on_clip_in_finished);
        self.deploy_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::on_deploy_finished);
        if self.reload_part {
            self.clip_part_in_audio
                .signals()
                .finished()
                .connect_obj(&gd, Self::on_clip_part_in_finished);
        }
        if self.pull_after_reload {
            self.reload_bolt_pull_audio
                .signals()
                .finished()
                .connect_obj(&gd, Self::on_bolt_pull_finished);
        }
        if self.bullet_scenes.is_empty() {
            #[allow(clippy::borrow_interior_mutable_const)]
            self.bullet_scenes.push(&*BULLET);
        }
    }
}

#[godot_api]
impl RustWeapon {
    #[signal]
    pub fn sig();

    pub fn deploy(&mut self) {
        self.deploy_audio.play();
    }

    pub fn weapon_ready(&mut self) {
        self.state = WeaponState::Ready;
    }

    pub fn update_ammo_hud(&self) {
        RustHUD::get().call_deferred(
            "update_ammo_hud",
            &[self.ammo.to_variant(), self.clip.to_variant()],
        );
    }

    pub fn update_jitter_hud(&self) {
        RustHUD::get().call_deferred("update_jitter_hud", &[self.current_jitter.to_variant()]);
    }

    pub fn fire(
        &mut self,
        player_damage: i64,
        player_distance: real,
        player_penetrate: real,
        player_repel: real,
    ) {
        if 0 == self.ammo
            || self.current_fire_cooldown > 0.0
            || WeaponState::Reloading == self.state && !self.reload_part
            || self.deploy_audio.is_playing()
            || self.fire_bolt_pull_audio.is_playing()
            || self.clip_part_in_audio.is_playing()
        {
            return;
        }
        let mut rng = rand::thread_rng();
        let vec: Vec<Gd<PackedScene>> = self.bullet_scenes.iter_shared().collect();
        for bullet_scene in vec {
            let r = if self.explode {
                //散弹枪一次性从所有子弹点射出子弹
                let mut r = Err(std::io::Error::other(
                    "Failed to instantiate bullet or grenade",
                ));
                for bullet_point in self.bullet_points.get_children().iter_shared() {
                    r = self.do_fire(
                        player_damage,
                        player_distance,
                        player_penetrate,
                        player_repel,
                        &bullet_scene,
                        bullet_point.cast::<Node2D>().get_global_position(),
                        self.get_random_direction(&mut rng, self.current_jitter),
                    );
                    if r.is_err() {
                        break;
                    }
                }
                r
            } else {
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
                    self.get_random_direction(&mut rng, self.current_jitter),
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
            if self.pull_after_fire && self.ammo > 1 {
                if let Some(mut tree) = self.base().get_tree() {
                    if let Some(mut timer) = tree.create_timer(0.5) {
                        timer.connect("timeout", &self.base().callable("on_fire_finished"));
                    }
                }
            }
            self.fire_audio.play();
            self.current_fire_cooldown = self.fire_cooldown;
            self.ammo -= 1;
            self.state = WeaponState::Firing;
            self.update_ammo_hud();
            if !self.silenced {
                //武器未消音
                NOISE_POSITION.store(self.bullet_points.get_global_position());
                if let Some(mut tree) = self.base().get_tree() {
                    if let Some(mut timer) = tree.create_timer(5.0) {
                        timer.connect(
                            "timeout",
                            &Callable::from_sync_fn("clean_weapon_noise", |_| {
                                NOISE_POSITION.store(NO_NOISE);
                                Ok(().to_variant())
                            }),
                        );
                    }
                }
            } else {
                NOISE_POSITION.store(NO_NOISE);
            }
        }
    }

    fn get_random_direction(&self, rng: &mut ThreadRng, jitter: real) -> Vector2 {
        //增加子弹散射
        self.base()
            .get_global_position()
            .direction_to(
                self.get_mouse_position()
                    + Vector2::new(
                        rng.gen_range(-jitter..=jitter),
                        rng.gen_range(-jitter..=jitter),
                    ),
            )
            .normalized()
    }

    #[allow(clippy::too_many_arguments)]
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
            gd_mut.set_speed(self.speed);
            gd_mut.set_bullet_point(bullet_point);
            gd_mut.set_final_distance(player_distance + self.distance);
            gd_mut.set_final_damage(player_damage.saturating_add(self.damage));
            gd_mut.set_final_penetrate(player_penetrate + self.penetrate);
            gd_mut.set_final_repel(player_repel + self.repel);
            gd_mut.set_direction(direction);
            drop(gd_mut);
            if let Some(mut parent) = RustPlayer::get().get_parent() {
                parent.add_child(&bullet);
                if self.jitter > 0.0 {
                    self.current_jitter =
                        (self.current_jitter + self.jitter / 5.0).min(self.jitter);
                    self.current_jitter_cooldown = self.fire_cooldown;
                    self.update_jitter_hud();
                }
                return Ok(());
            }
        } else if let Some(mut grenade) = bullet_scene.try_instantiate_as::<RustGrenade>() {
            grenade.set_global_position(bullet_point);
            let mut gd_mut = grenade.bind_mut();
            gd_mut.set_speed(self.speed);
            gd_mut.set_bullet_point(bullet_point);
            gd_mut.set_final_distance(player_distance + self.distance);
            gd_mut.set_final_damage(player_damage.saturating_add(self.damage));
            gd_mut.set_final_repel(player_repel + self.repel);
            gd_mut.set_direction(direction);
            drop(gd_mut);
            if let Some(mut parent) = RustPlayer::get().get_parent() {
                parent.add_child(&grenade);
                if let Some(mut rocket) = self.base().try_get_node_as::<Sprite2D>("Rocket") {
                    rocket.set_visible(false);
                }
                return Ok(());
            }
        }
        Err(std::io::Error::other(
            "Failed to instantiate bullet or grenade",
        ))
    }

    #[func]
    pub fn on_deploy_finished(&mut self) {
        if self.pull_after_deploy {
            self.fire_bolt_pull_audio.play();
        }
        RustPlayer::get().call_deferred("zoom", &[]);
    }

    #[func]
    pub fn on_fire_finished(&mut self) {
        self.fire_bolt_pull_audio.play();
    }

    pub fn reload(&mut self) -> bool {
        if self.clip == self.ammo
            || WeaponState::Reloading == self.state
            || self.clip_out_audio.is_playing()
            || self.reload_part && self.clip_part_in_audio.is_playing()
            || self.clip_in_audio.is_playing()
            || self.pull_after_reload && self.reload_bolt_pull_audio.is_playing()
        {
            return false;
        }
        self.reloading =
            self.reload_time - self.clip_in_audio.get_stream().unwrap().get_length() as real;
        if self.pull_after_reload {
            self.reloading -= self
                .reload_bolt_pull_audio
                .get_stream()
                .unwrap()
                .get_length() as real;
        }
        if self.reload_part {
            self.part_reload_time = ((self.reloading
                - self.clip_out_audio.get_stream().unwrap().get_length() as real)
                / self.clip as real)
                .max(self.clip_part_in_audio.get_stream().unwrap().get_length() as real);
            self.reloading = self.clip_out_audio.get_stream().unwrap().get_length() as real
                + self.part_reload_time;
        }
        self.reloading = self.reloading.max(0.0);
        self.clip_out_audio.play();
        self.state = WeaponState::Reloading;
        true
    }

    #[func]
    pub fn on_clip_out_finished(&mut self) {
        if self.reload_part {
            if !self.clip_part_in_audio.is_playing() {
                self.clip_part_in_audio.play();
            }
            return;
        }
        if WeaponState::Reloading != self.state
            || self.reloading > 0.0
            || self.clip_in_audio.is_playing()
        {
            return;
        }
        self.clip_in_audio.play();
    }

    #[func]
    pub fn on_clip_part_in_finished(&mut self) {
        if WeaponState::Reloading != self.state
            || self.reloading > 0.0
            || self.clip_part_in_audio.is_playing()
        {
            return;
        }
        self.reloading = self.part_reload_time;
        self.ammo += 1;
        self.ammo = self.ammo.min(self.clip);
        self.update_ammo_hud();
        RustPlayer::get().call_deferred("reloading", &[]);
        if self.ammo == self.clip {
            self.clip_in_audio.play();
            return;
        }
        self.clip_part_in_audio.play();
    }

    #[func]
    pub fn on_clip_in_finished(&mut self) {
        if WeaponState::Reloading != self.state {
            return;
        }
        if self.pull_after_reload {
            self.reload_bolt_pull_audio.play();
            return;
        }
        // 无需拉栓
        self.on_bolt_pull_finished();
    }

    #[func]
    pub fn on_bolt_pull_finished(&mut self) {
        if let Some(mut rocket) = self.base().try_get_node_as::<Sprite2D>("Rocket") {
            rocket.set_visible(true);
        }
        self.weapon_ready();
        self.reloading = 0.0;
        self.current_fire_cooldown = 0.0;
        self.current_jitter = 0.0;
        self.current_jitter_cooldown = 0.0;
        self.ammo = self.clip;
        self.update_jitter_hud();
        self.update_ammo_hud();
        RustPlayer::get().call_deferred("reloaded", &[]);
    }

    pub fn stop_reload(&mut self) {
        self.deploy_audio.stop();
        self.fire_bolt_pull_audio.stop();
        self.clip_out_audio.stop();
        self.clip_part_in_audio.stop();
        self.clip_in_audio.stop();
        self.reload_bolt_pull_audio.stop();
        self.weapon_ready();
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

    pub fn get_noise_position() -> Option<Vector2> {
        let r = NOISE_POSITION.load();
        if NO_NOISE == r { None } else { Some(r) }
    }
}
