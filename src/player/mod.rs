use crate::common::RustMessage;
use crate::player::hud::PlayerHUD;
use crate::weapon::RustWeapon;
use crate::world::RustWorld;
use crate::{
    DEFAULT_SCREEN_SIZE, PLAYER_LEVEL_UP_BARRIER, PLAYER_LEVEL_UP_GROW_RATE, PLAYER_MAX_HEALTH,
    PLAYER_MAX_LIVES, PLAYER_MOVE_SPEED, PlayerState, PlayerUpgrade, random_bool,
};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Vector2, real};
use godot::classes::node::PhysicsInterpolationMode;
use godot::classes::{
    AnimatedSprite2D, AudioStreamPlayer2D, CharacterBody2D, DisplayServer, GpuParticles2D,
    ICharacterBody2D, Input, InputEvent, Node2D, PackedScene,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod hud;

static POSITION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

static STATE: AtomicCell<PlayerState> = AtomicCell::new(PlayerState::Born);

static RELOADING: AtomicCell<real> = AtomicCell::new(0.0);

static IMPACT_POSITION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

static IMPACTING: AtomicCell<f64> = AtomicCell::new(0.0);

static KILL_COUNT: AtomicU32 = AtomicU32::new(0);

static KILL_BOSS_COUNT: AtomicU32 = AtomicU32::new(0);

static SCORE: AtomicU64 = AtomicU64::new(0);

static DIED: AtomicU64 = AtomicU64::new(0);

static LAST_SCORE_UPDATE: AtomicCell<f64> = AtomicCell::new(0.0);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPlayer {
    // 玩家无敌
    #[export]
    invincible: bool,
    #[export]
    current_weapon_index: i32,
    // 玩家穿透
    #[export]
    lives: u32,
    // 玩家伤害
    #[export]
    damage: i64,
    // 玩家射程
    #[export]
    distance: real,
    // 玩家穿透
    #[export]
    penetrate: real,
    // 玩家击退
    #[export]
    repel: real,
    // 玩家最大生命值
    #[export]
    health: u32,
    // 玩家移动速度
    #[export]
    speed: real,
    #[export]
    level_up_barrier: u32,
    current_level_up_barrier: u64,
    current_lives: u32,
    current_health: u32,
    state: PlayerState,
    current_speed: real,
    animated_sprite2d: OnReady<Gd<AnimatedSprite2D>>,
    weapons: OnReady<Gd<Node2D>>,
    blood_flash: OnReady<Gd<GpuParticles2D>>,
    hud: OnReady<Gd<PlayerHUD>>,
    run_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    body_hurt: OnReady<Gd<AudioStreamPlayer2D>>,
    bone_hurt: OnReady<Gd<AudioStreamPlayer2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    change_success_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    change_fail_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    level_up_scene: OnReady<Gd<PackedScene>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            invincible: false,
            current_weapon_index: 0,
            lives: PLAYER_MAX_LIVES,
            damage: 0,
            distance: 0.0,
            penetrate: 0.0,
            repel: 0.0,
            health: PLAYER_MAX_HEALTH,
            current_health: PLAYER_MAX_HEALTH,
            state: PlayerState::Born,
            speed: PLAYER_MOVE_SPEED,
            level_up_barrier: PLAYER_LEVEL_UP_BARRIER,
            current_level_up_barrier: PLAYER_LEVEL_UP_BARRIER as u64,
            current_lives: PLAYER_MAX_LIVES,
            current_speed: PLAYER_MOVE_SPEED,
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            weapons: OnReady::from_node("Weapon"),
            blood_flash: OnReady::from_node("GpuParticles2D"),
            hud: OnReady::from_node("PlayerHUD"),
            run_audio: OnReady::from_node("RunAudio"),
            body_hurt: OnReady::from_node("HurtAudio1"),
            bone_hurt: OnReady::from_node("HurtAudio2"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            change_success_audio: OnReady::from_node("ChangeWeaponSuccess"),
            change_fail_audio: OnReady::from_node("ChangeWeaponFail"),
            level_up_scene: OnReady::from_loaded("res://scenes/rust_message.tscn"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        if PlayerState::Dead == self.state || RustWorld::is_paused() {
            return;
        }
        self.level_up();
        let mut hud = self.hud.bind_mut();
        hud.update_killed_hud();
        hud.update_score_hud();
        hud.update_died_hud();
        drop(hud);
        if PlayerState::Reload == self.state {
            // 选择这种计时的方式是为了支持打断换弹
            let reload_cost = RELOADING.load() + delta as real;
            RELOADING.store(reload_cost);
            if reload_cost >= self.get_current_weapon().bind().get_reload_time() {
                self.reloaded();
            }
        } else if PlayerState::Impact == self.state {
            let impact_cost = IMPACTING.load() + delta;
            IMPACTING.store(impact_cost);
            if impact_cost < 1.0 {
                self.impacting();
            } else {
                self.impacted();
            }
        }
        let player_position = self.base().get_global_position();
        POSITION.store(player_position);
        let mouse_position = self.get_mouse_position();
        self.base_mut().look_at(mouse_position);
        let input = Input::singleton();
        if input.is_action_pressed("mouse_left") {
            self.shoot();
        } else if (input.is_action_pressed("shift") || input.is_action_pressed("mouse_right"))
            && (input.is_action_pressed("move_left")
                || input.is_action_pressed("move_right")
                || input.is_action_pressed("move_up")
                || input.is_action_pressed("move_down"))
        {
            self.run();
        }
        let mut move_direction = Vector2::new(
            input.get_axis("move_left", "move_right"),
            input.get_axis("move_up", "move_down"),
        );
        match self.state {
            PlayerState::Run => self
                .animated_sprite2d
                .look_at(player_position + move_direction),
            PlayerState::Impact => {
                move_direction = IMPACT_POSITION
                    .load()
                    .direction_to(player_position)
                    .normalized();
                self.animated_sprite2d.look_at(IMPACT_POSITION.load());
            }
            _ => self.animated_sprite2d.look_at(mouse_position),
        }
        let mut character_body2d = self.base.to_gd();
        if move_direction != Vector2::ZERO {
            character_body2d.set_velocity(move_direction.normalized() * self.current_speed);
        } else {
            character_body2d.set_velocity(Vector2::ZERO);
            self.guard();
        }
        character_body2d.move_and_slide();
    }

    fn enter_tree(&mut self) {
        self.scale();
    }

    fn ready(&mut self) {
        self.change_weapon(self.current_weapon_index);
        self.base_mut()
            .set_physics_interpolation_mode(PhysicsInterpolationMode::ON);
        let rust_weapon = self.get_current_weapon();
        let mut hud = self.hud.bind_mut();
        hud.update_lives_hud(self.current_lives, self.lives);
        hud.update_hp_hud(self.current_health, self.health);
        hud.update_ammo_hud(rust_weapon.bind().get_ammo(), rust_weapon.bind().get_clip());
        hud.update_damage_hud(self.damage.saturating_add(rust_weapon.bind().get_damage()));
        hud.update_distance_hud(self.distance + rust_weapon.bind().get_distance());
        hud.update_repel_hud(self.repel + rust_weapon.bind().get_repel());
        hud.update_penetrate_hud(self.penetrate + rust_weapon.bind().get_penetrate());
        hud.update_killed_hud();
        hud.update_score_hud();
        hud.update_died_hud();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if RustWorld::is_paused() {
            return;
        }
        if event.is_action_pressed("r") {
            self.reload();
        } else if event.is_action_released("shift")
            || event.is_action_released("mouse_left")
            || event.is_action_released("mouse_right")
        {
            self.guard();
        } else if event.is_action_pressed("1") {
            self.change_weapon(0);
        } else if event.is_action_pressed("2") {
            self.change_weapon(1);
        } else if event.is_action_pressed("3") {
            self.change_weapon(2);
        }
    }
}

#[godot_api]
impl RustPlayer {
    pub fn scale(&self) {
        //计算缩放倍数
        let window_size = DisplayServer::singleton()
            .screen_get_size_ex()
            .screen(DisplayServer::SCREEN_PRIMARY)
            .done();
        let scale = (window_size.x as real / DEFAULT_SCREEN_SIZE.x)
            .min(window_size.y as real / DEFAULT_SCREEN_SIZE.y)
            .max(1.0);
        self.base()
            .get_window()
            .unwrap()
            .set_content_scale_factor(scale);
    }

    pub fn on_hit(&mut self, hit_val: i64, hit_position: Vector2) {
        if !self.invincible {
            let health = self.current_health;
            self.current_health = if hit_val > 0 {
                health.saturating_sub(hit_val as u32)
            } else {
                health.saturating_add(-hit_val as u32)
            };
            self.hud
                .bind_mut()
                .update_hp_hud(self.current_health, self.health);
        }
        if 0 != self.current_health {
            self.hit(hit_position);
        } else {
            self.die(hit_position);
        }
    }

    pub fn reborn(&mut self) {
        self.current_lives = self.lives.saturating_add(1);
        self.hud
            .bind_mut()
            .update_lives_hud(self.current_lives, self.lives);
        self.born();
    }

    #[func]
    pub fn born(&mut self) {
        if PlayerState::Dead != self.state || 0 == self.current_lives {
            return;
        }
        self.current_lives -= 1;
        self.weapons.set_visible(true);
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed;
        self.state = PlayerState::Born;
        self.current_health = self.health;
        STATE.store(self.state);
        self.hud
            .bind_mut()
            .update_lives_hud(self.current_lives, self.lives);
        self.hud
            .bind_mut()
            .update_hp_hud(self.current_health, self.health);
    }

    #[func]
    pub fn guard(&mut self) {
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || PlayerState::Reload == self.state
        {
            return;
        }
        self.weapons.set_visible(true);
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed;
        self.state = PlayerState::Guard;
        STATE.store(self.state);
    }

    pub fn run(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Impact == self.state {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.5;
        self.state = PlayerState::Run;
        STATE.store(self.state);
        //打断换弹
        RELOADING.store(0.0);
        if !self.run_audio.is_playing() {
            self.run_audio.play();
        }
    }

    pub fn shoot(&mut self) {
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || PlayerState::Reload == self.state
        {
            return;
        }
        let mut rust_weapon = self.get_current_weapon();
        if rust_weapon.bind().must_reload() {
            // 没子弹时自动装填
            self.reload();
            return;
        }
        rust_weapon.set_visible(true);
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.5;
        self.state = PlayerState::Shoot;
        STATE.store(self.state);
        rust_weapon
            .bind_mut()
            .fire(self.damage, self.distance, self.penetrate, self.repel);
        self.hud
            .bind_mut()
            .update_ammo_hud(rust_weapon.bind().get_ammo(), rust_weapon.bind().get_clip());
    }

    pub fn reload(&mut self) {
        if PlayerState::Dead == self.state
            || PlayerState::Impact == self.state
            || !self.get_current_weapon().bind_mut().reload()
        {
            return;
        }
        self.weapons.set_visible(true);
        self.animated_sprite2d.play_ex().name("reload").done();
        self.current_speed = self.speed * 0.75;
        self.state = PlayerState::Reload;
        STATE.store(self.state);
    }

    pub fn reloaded(&mut self) {
        if PlayerState::Dead == self.state || PlayerState::Impact == self.state {
            return;
        }
        self.state = PlayerState::Guard;
        let mut rust_weapon = self.get_current_weapon();
        let clip = rust_weapon.bind_mut().reloaded();
        self.hud
            .bind_mut()
            .update_ammo_hud(rust_weapon.bind().get_ammo(), clip);
        self.guard();
        RELOADING.store(0.0);
    }

    pub fn change_weapon(&mut self, weapon_index: i32) {
        if PlayerState::Dead == self.state || PlayerState::Impact == self.state {
            return;
        }
        for i in 0..self.weapons.get_child_count() {
            if let Some(node) = self.weapons.get_child(i) {
                let mut weapon = node.cast::<RustWeapon>();
                if weapon_index == i {
                    weapon.set_visible(true);
                } else {
                    weapon.set_visible(false);
                }
            }
        }
        self.weapons.set_visible(true);
        if weapon_index == self.current_weapon_index {
            self.change_fail_audio.play();
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.75;
        self.current_weapon_index = weapon_index;
        self.state = PlayerState::Guard;
        STATE.store(self.state);
        //打断换弹
        RELOADING.store(0.0);
        self.change_success_audio.play();
        // 更新HUD
        let rust_weapon = self.get_current_weapon();
        let mut hud = self.hud.bind_mut();
        hud.update_ammo_hud(rust_weapon.bind().get_ammo(), rust_weapon.bind().get_clip());
        hud.update_damage_hud(self.damage.saturating_add(rust_weapon.bind().get_damage()));
        hud.update_distance_hud(self.distance + rust_weapon.bind().get_distance());
        hud.update_repel_hud(self.repel + rust_weapon.bind().get_repel());
        hud.update_penetrate_hud(self.penetrate + rust_weapon.bind().get_penetrate());
    }

    pub fn hit(&mut self, hit_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.play_ex().name("hit").done();
        self.current_speed = self.speed * 0.5;
        self.state = PlayerState::Hit;
        let player_position = self.base().get_global_position();
        self.blood_flash.set_global_position(
            player_position + player_position.direction_to(hit_position).normalized() * 18.0,
        );
        self.blood_flash.look_at(hit_position);
        self.blood_flash.restart();
        STATE.store(self.state);
        if random_bool() {
            self.body_hurt.play();
        } else {
            self.bone_hurt.play();
        }
        if !self.scream_audio.is_playing() {
            self.scream_audio.play();
        }
    }

    pub fn on_impact(&mut self, hit_val: i64, impact_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.on_hit(hit_val, impact_position);
        if PlayerState::Dead == self.state {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.play_ex().name("bump").done();
        self.current_speed = self.speed * 1.25;
        self.state = PlayerState::Impact;
        let player_position = self.base().get_global_position();
        self.blood_flash.set_global_position(
            player_position + player_position.direction_to(impact_position).normalized() * 18.0,
        );
        self.blood_flash.look_at(impact_position);
        self.blood_flash.set_one_shot(false);
        self.blood_flash.set_emitting(true);
        self.blood_flash.restart();
        STATE.store(self.state);
        IMPACT_POSITION.store(impact_position);
        STATE.store(self.state);
    }

    pub fn impacting(&mut self) {
        if PlayerState::Impact != self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("bump").done();
        self.current_speed = self.speed * 1.25;
        self.state = PlayerState::Impact;
        STATE.store(self.state);
        let hit_position = IMPACT_POSITION.load();
        self.base_mut().look_at(hit_position);
        if !self.scream_audio.is_playing() {
            self.scream_audio.play();
        }
    }

    pub fn impacted(&mut self) {
        if PlayerState::Impact != self.state {
            return;
        }
        self.state = PlayerState::Guard;
        self.blood_flash.set_one_shot(true);
        self.blood_flash.set_emitting(false);
        IMPACT_POSITION.store(Vector2::ZERO);
        IMPACTING.store(0.0);
        self.guard();
    }

    pub fn die(&mut self, hit_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.weapons.set_visible(false);
        self.animated_sprite2d.look_at(hit_position);
        self.animated_sprite2d.play_ex().name("die").done();
        self.current_speed = 0.0;
        self.state = PlayerState::Dead;
        STATE.store(self.state);
        self.die_audio.play();
        DIED.fetch_add(1, Ordering::Release);
        if 0 == self.current_lives {
            if let Some(tree) = self.base().get_tree() {
                if let Some(root) = tree.get_root() {
                    root.get_node_as::<RustWorld>("RustWorld")
                        .signals()
                        .player_dead()
                        .emit();
                }
            }
            return;
        }
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(3.0) {
                timer.connect("timeout", &self.base().callable("born"));
            }
        }
    }

    pub fn level_up(&mut self) {
        if Self::get_score() < self.current_level_up_barrier {
            return;
        }
        //防止重复触发升级
        let damage = self
            .damage
            .saturating_add(self.get_current_weapon().bind().get_damage());
        RustPlayer::add_score(damage as u64);
        self.level_up_barrier = (self.level_up_barrier as real * PLAYER_LEVEL_UP_GROW_RATE) as u32;
        self.current_level_up_barrier += self.level_up_barrier as u64;
        RustWorld::pause();
        self.hud.bind_mut().set_upgrade_visible(true);
    }

    #[func]
    pub fn upgrade_penetrate(&mut self) {
        //穿透力升级
        self.penetrate += 0.1;
        let new_penetrate = self.penetrate + self.get_current_weapon().bind().get_penetrate();
        self.hud.bind_mut().update_penetrate_hud(new_penetrate);
        self.show_upgrade_label(PlayerUpgrade::Penetrate);
    }

    #[func]
    pub fn upgrade_damage(&mut self) {
        //伤害升级
        self.damage = self.damage.saturating_add(2);
        let new_damage = self
            .damage
            .saturating_add(self.get_current_weapon().bind().get_damage());
        self.hud.bind_mut().update_damage_hud(new_damage);
        self.show_upgrade_label(PlayerUpgrade::Damage);
    }

    #[func]
    pub fn upgrade_repel(&mut self) {
        //击退力升级
        self.repel += 1.0;
        let new_repel = self.repel + self.get_current_weapon().bind().get_repel();
        self.hud.bind_mut().update_repel_hud(new_repel);
        self.show_upgrade_label(PlayerUpgrade::Repel);
    }

    #[func]
    pub fn upgrade_lives(&mut self) {
        //奖励生命数
        self.lives = self.lives.saturating_add(1);
        self.current_lives = self.current_lives.saturating_add(1);
        self.hud
            .bind_mut()
            .update_lives_hud(self.current_lives, self.lives);
        self.show_upgrade_label(PlayerUpgrade::Lives);
    }

    #[func]
    pub fn upgrade_distance(&mut self) {
        //射击距离升级
        self.distance += 20.0;
        let new_distance = self.distance + self.get_current_weapon().bind().get_distance();
        self.hud.bind_mut().update_distance_hud(new_distance);
        self.show_upgrade_label(PlayerUpgrade::Distance);
    }

    #[func]
    pub fn upgrade_health(&mut self) {
        //生命值升级
        self.health = self.health.saturating_add(10);
        self.current_health = self.current_health.saturating_add(10);
        self.hud
            .bind_mut()
            .update_hp_hud(self.current_health, self.health);
        self.show_upgrade_label(PlayerUpgrade::Health);
    }

    fn show_upgrade_label(&mut self, what: PlayerUpgrade) {
        self.hud.bind_mut().set_upgrade_visible(false);
        if let Some(mut level_up_label) = self.level_up_scene.try_instantiate_as::<RustMessage>() {
            level_up_label.set_global_position(RustPlayer::get_position());
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&level_up_label);
                    level_up_label.bind_mut().show_level_up(what);
                }
            }
        }
        RustWorld::resume();
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        self.base().get_canvas_transform().affine_inverse()
            * self
                .base()
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }

    pub fn get_current_weapon(&self) -> Gd<RustWeapon> {
        self.weapons
            .get_child(self.current_weapon_index)
            .expect("Weapon not configured")
            .cast::<RustWeapon>()
    }

    pub fn add_kill_count() {
        KILL_COUNT.fetch_add(1, Ordering::Release);
    }

    pub fn get_kill_count() -> u32 {
        KILL_COUNT.load(Ordering::Acquire)
    }

    pub fn add_kill_boss_count() {
        KILL_BOSS_COUNT.fetch_add(1, Ordering::Release);
    }

    pub fn get_kill_boss_count() -> u32 {
        KILL_BOSS_COUNT.load(Ordering::Acquire)
    }

    pub fn add_score(score: u64) {
        SCORE.fetch_add(score, Ordering::Release);
        Self::reset_last_score_update();
    }

    pub fn get_score() -> u64 {
        SCORE.load(Ordering::Acquire)
    }

    pub fn get_died() -> u64 {
        DIED.load(Ordering::Acquire)
    }

    pub fn get_last_score_update() -> f64 {
        LAST_SCORE_UPDATE.load()
    }

    pub fn reset_last_score_update() {
        LAST_SCORE_UPDATE.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("1970-01-01 00:00:00 UTC was {} seconds ago!")
                .as_secs_f64(),
        );
    }

    pub fn get_position() -> Vector2 {
        POSITION.load()
    }

    pub fn get_state() -> PlayerState {
        STATE.load()
    }
}
