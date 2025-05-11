use crate::player::hud::PlayerHUD;
use crate::player::level_up::PlayerLevelUp;
use crate::weapon::RustWeapon;
use crate::world::RustWorld;
use crate::{
    MAX_AMMO, PLAYER_LEVEL_UP_BARRIER, PLAYER_LEVEL_UP_GROW_RATE, PLAYER_MAX_HEALTH,
    PLAYER_MAX_LIVES, PLAYER_MOVE_SPEED, PlayerState, PlayerUpgrade,
};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Vector2, real};
use godot::classes::node::PhysicsInterpolationMode;
use godot::classes::{
    AnimatedSprite2D, AudioStreamPlayer2D, CharacterBody2D, GpuParticles2D, ICharacterBody2D,
    Input, InputEvent, Node2D, PackedScene,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub mod hud;

pub mod level_up;

static POSITION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

static STATE: AtomicCell<PlayerState> = AtomicCell::new(PlayerState::Born);

static RELOADING: AtomicCell<f64> = AtomicCell::new(0.0);

static KILL_COUNT: AtomicU32 = AtomicU32::new(0);

static KILL_BOSS_COUNT: AtomicU32 = AtomicU32::new(0);

static SCORE: AtomicU64 = AtomicU64::new(0);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPlayer {
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
    weapon: OnReady<Gd<Node2D>>,
    blood_flash: OnReady<Gd<GpuParticles2D>>,
    hud: OnReady<Gd<PlayerHUD>>,
    run_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hurt_audio1: OnReady<Gd<AudioStreamPlayer2D>>,
    hurt_audio2: OnReady<Gd<AudioStreamPlayer2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    level_up_scene: OnReady<Gd<PackedScene>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
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
            weapon: OnReady::from_node("Weapon"),
            blood_flash: OnReady::from_node("GpuParticles2D"),
            hud: OnReady::from_node("PlayerHUD"),
            run_audio: OnReady::from_node("RunAudio"),
            hurt_audio1: OnReady::from_node("HurtAudio1"),
            hurt_audio2: OnReady::from_node("HurtAudio2"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            level_up_scene: OnReady::from_loaded("res://scenes/player_level_up.tscn"),
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut()
            .set_physics_interpolation_mode(PhysicsInterpolationMode::ON);
        let rust_weapon = self.weapon.get_node_as::<RustWeapon>("RustWeapon");
        let mut hud = self.hud.bind_mut();
        hud.update_lives_hud(self.current_lives, self.lives);
        hud.update_hp_hud(self.current_health, self.health);
        hud.update_ammo_hud(rust_weapon.bind().get_ammo(), MAX_AMMO);
        hud.update_damage_hud(self.damage.saturating_add(rust_weapon.bind().get_damage()));
        hud.update_distance_hud(self.distance + rust_weapon.bind().get_distance());
        hud.update_repel_hud(self.repel + rust_weapon.bind().get_repel());
        hud.update_penetrate_hud(self.penetrate + rust_weapon.bind().get_penetrate());
        hud.update_killed_hud();
        hud.update_score_hud();
    }

    fn process(&mut self, delta: f64) {
        if PlayerState::Dead == self.state || RustWorld::is_paused() {
            return;
        }
        self.level_up();
        let mut hud = self.hud.bind_mut();
        hud.update_killed_hud();
        hud.update_score_hud();
        drop(hud);
        if PlayerState::Reload == self.state {
            let reload_cost = RELOADING.load() + delta;
            RELOADING.store(reload_cost);
            if reload_cost >= self.get_rust_weapon().bind().get_reload_time() as f64 / 1000.0 {
                self.reloaded();
            }
        }
        let player_position = self.base().get_global_position();
        POSITION.store(player_position);
        let mouse_position = self.get_mouse_position();
        self.weapon.look_at(mouse_position);
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
        let key_direction = Vector2::new(
            input.get_axis("move_left", "move_right"),
            input.get_axis("move_up", "move_down"),
        );
        match self.state {
            PlayerState::Run => self
                .animated_sprite2d
                .look_at(player_position + key_direction),
            _ => self.animated_sprite2d.look_at(mouse_position),
        }
        let mut character_body2d = self.base.to_gd();
        if key_direction != Vector2::ZERO {
            character_body2d.set_velocity(key_direction.normalized() * self.current_speed);
        } else {
            character_body2d.set_velocity(Vector2::ZERO);
            self.guard();
        }
        character_body2d.move_and_slide();
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
        }
    }
}

#[godot_api]
impl RustPlayer {
    #[func]
    pub fn on_hit(&mut self, hit_val: i64, hit_position: Vector2) {
        let health = self.current_health;
        self.current_health = if hit_val > 0 {
            health.saturating_sub(hit_val as u32)
        } else {
            health.saturating_add(-hit_val as u32)
        };
        self.hud
            .bind_mut()
            .update_hp_hud(self.current_health, self.health);
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
        if PlayerState::Dead == self.state || PlayerState::Reload == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed;
        self.state = PlayerState::Guard;
        STATE.store(self.state);
    }

    pub fn run(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
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
        if PlayerState::Dead == self.state || PlayerState::Reload == self.state {
            return;
        }
        let mut rust_weapon = self.get_rust_weapon();
        if rust_weapon.bind().must_reload() {
            // 没子弹时自动装填
            self.reload();
            return;
        }
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
        if PlayerState::Dead == self.state || !self.get_rust_weapon().bind_mut().reload() {
            return;
        }
        self.animated_sprite2d.play_ex().name("reload").done();
        self.current_speed = self.speed * 0.75;
        self.state = PlayerState::Reload;
        STATE.store(self.state);
    }

    #[func]
    pub fn reloaded(&mut self) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.state = PlayerState::Guard;
        let mut rust_weapon = self.get_rust_weapon();
        let clip = rust_weapon.bind_mut().reloaded();
        self.hud
            .bind_mut()
            .update_ammo_hud(rust_weapon.bind().get_ammo(), clip);
        self.guard();
        RELOADING.store(0.0);
    }

    pub fn hit(&mut self, hit_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
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
        if Self::random_bool() {
            self.hurt_audio1.play();
        } else {
            self.hurt_audio2.play();
        }
        if !self.scream_audio.is_playing() {
            self.scream_audio.play();
        }
    }

    pub fn die(&mut self, hit_position: Vector2) {
        if PlayerState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.look_at(hit_position);
        self.animated_sprite2d.play_ex().name("die").done();
        self.current_speed = 0.0;
        self.state = PlayerState::Dead;
        STATE.store(self.state);
        self.die_audio.play();
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
            .saturating_add(self.get_rust_weapon().bind().get_damage());
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
        let new_penetrate = self.penetrate + self.get_rust_weapon().bind().get_penetrate();
        self.hud.bind_mut().update_penetrate_hud(new_penetrate);
        self.show_upgrade_label(PlayerUpgrade::Penetrate);
    }

    #[func]
    pub fn upgrade_damage(&mut self) {
        //伤害升级
        self.damage = self.damage.saturating_add(2);
        let new_damage = self
            .damage
            .saturating_add(self.get_rust_weapon().bind().get_damage());
        self.hud.bind_mut().update_damage_hud(new_damage);
        self.show_upgrade_label(PlayerUpgrade::Damage);
    }

    #[func]
    pub fn upgrade_repel(&mut self) {
        //击退力升级
        self.repel += 1.0;
        let new_repel = self.repel + self.get_rust_weapon().bind().get_repel();
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
        let new_distance = self.distance + self.get_rust_weapon().bind().get_distance();
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
        if let Some(mut level_up_label) = self.level_up_scene.try_instantiate_as::<PlayerLevelUp>()
        {
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

    pub fn get_rust_weapon(&mut self) -> Gd<RustWeapon> {
        self.weapon.get_node_as::<RustWeapon>("RustWeapon")
    }

    pub fn random_bool() -> bool {
        rand::thread_rng().gen_range(-1.0..1.0) >= 0.0
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
    }

    pub fn get_score() -> u64 {
        SCORE.load(Ordering::Acquire)
    }

    pub fn get_position() -> Vector2 {
        POSITION.load()
    }

    pub fn get_state() -> PlayerState {
        STATE.load()
    }
}
