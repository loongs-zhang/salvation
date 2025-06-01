use crate::common::RustMessage;
use crate::grenade::RustGrenade;
use crate::hud::RustHUD;
use crate::knife::RustKnife;
use crate::world::RustWorld;
use crate::{
    GRENADE_DAMAGE, GRENADE_DISTANCE, GRENADE_REPEL, MESSAGE, PLAYER_LEVEL_UP_BARRIER,
    PLAYER_MAX_HEALTH, PLAYER_MAX_LIVES, PLAYER_MOVE_SPEED, PlayerState, scale_rate,
};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Array, GString, Vector2, real};
use godot::classes::node::PhysicsInterpolationMode;
use godot::classes::{
    AnimatedSprite2D, AudioStreamPlayer2D, Camera2D, CharacterBody2D, GpuParticles2D,
    ICharacterBody2D, Input, InputEvent, Label, Node2D, PackedScene, RemoteTransform2D,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use godot::tools::load;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod state;

pub mod weapon;

pub mod upgrade;

pub mod save;

static POSITION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

static LAST_SCORE_UPDATE: AtomicU64 = AtomicU64::new(0);

#[allow(clippy::declare_interior_mutable_const)]
const GRENADE: LazyLock<Gd<PackedScene>> =
    LazyLock::new(|| load("res://scenes/grenades/fgrenade.tscn"));

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPlayer {
    #[export]
    player_name: GString,
    // 玩家无敌
    #[export]
    invincible: bool,
    // 当前武器下标
    #[export]
    current_weapon_index: i32,
    // 玩家剩余生命条数
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
    // 升级所需的分数
    #[export]
    level_up_barrier: u32,
    #[export]
    grenade_cooldown: real,
    // 手雷类型
    #[export]
    grenade_scenes: Array<Gd<PackedScene>>,
    #[export]
    chop_cooldown: real,
    current_chop_cooldown: f64,
    current_grenade_cooldown: real,
    current_level_up_barrier: u32,
    current_lives: u32,
    current_health: u32,
    state: PlayerState,
    current_speed: real,
    impact_position: Vector2,
    left_impact_time: f64,
    // 玩家获得的分数
    score: u32,
    // 玩家死亡的次数
    died: u32,
    // 击杀普通僵尸数
    kill_count: u32,
    // 击杀BOSS数
    kill_boss_count: u32,
    remote_transform2d: OnReady<Gd<RemoteTransform2D>>,
    animated_sprite2d: OnReady<Gd<AnimatedSprite2D>>,
    camera: OnReady<Gd<Camera2D>>,
    knife: OnReady<Gd<RustKnife>>,
    weapons: OnReady<Gd<Node2D>>,
    blood_flash: OnReady<Gd<GpuParticles2D>>,
    hud: OnReady<Gd<RustHUD>>,
    run_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    body_hurt: OnReady<Gd<AudioStreamPlayer2D>>,
    bone_hurt: OnReady<Gd<AudioStreamPlayer2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    change_success_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    change_fail_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    zoom_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    headshot_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    grenade_point: OnReady<Gd<Node2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            player_name: GString::new(),
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
            grenade_cooldown: 10.0,
            current_grenade_cooldown: 0.0,
            grenade_scenes: Array::new(),
            chop_cooldown: 0.5,
            current_chop_cooldown: 0.0,
            current_level_up_barrier: PLAYER_LEVEL_UP_BARRIER,
            current_lives: PLAYER_MAX_LIVES,
            current_speed: PLAYER_MOVE_SPEED,
            impact_position: Vector2::ZERO,
            left_impact_time: 0.0,
            score: 0,
            died: 0,
            kill_count: 0,
            kill_boss_count: 0,
            remote_transform2d: OnReady::from_node("RemoteTransform2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            camera: OnReady::from_node("Camera2D"),
            knife: OnReady::from_node("Knife"),
            weapons: OnReady::from_node("Weapon"),
            blood_flash: OnReady::from_node("GpuParticles2D"),
            hud: OnReady::from_node("RustHUD"),
            run_audio: OnReady::from_node("RunAudio"),
            body_hurt: OnReady::from_node("HurtAudio1"),
            bone_hurt: OnReady::from_node("HurtAudio2"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            change_success_audio: OnReady::from_node("ChangeWeaponSuccess"),
            change_fail_audio: OnReady::from_node("ChangeWeaponFail"),
            zoom_audio: OnReady::from_node("ZoomAudio"),
            headshot_audio: OnReady::from_node("HeadshotAudio"),
            grenade_point: OnReady::from_node("GrenadePoint"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        self.hud.bind_mut().update_fps_hud();
        if self.remote_transform2d.is_instance_valid() {
            self.remote_transform2d.set_global_rotation_degrees(0.0);
        }
        if PlayerState::Dead == self.state || RustWorld::is_paused() {
            return;
        }
        self.current_grenade_cooldown -= delta as real;
        self.current_chop_cooldown -= delta;
        self.level_up();
        let mut hud = self.hud.bind_mut();
        hud.update_killed_hud(self.kill_boss_count, self.kill_count);
        hud.update_score_hud(self.score);
        hud.update_died_hud(self.died);
        drop(hud);
        if PlayerState::Impact == self.state {
            self.left_impact_time += delta;
            if self.left_impact_time < 1.0 {
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
        } else if input.is_action_pressed("e") {
            self.chop();
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
                move_direction = self
                    .impact_position
                    .direction_to(player_position)
                    .normalized();
                self.animated_sprite2d.look_at(self.impact_position);
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

    fn exit_tree(&mut self) {
        self.grenade_scenes.clear();
    }

    fn ready(&mut self) {
        self.knife.set_visible(false);
        self.change_weapon(self.current_weapon_index);
        self.base_mut()
            .set_physics_interpolation_mode(PhysicsInterpolationMode::ON);
        let rust_weapon = self.get_current_weapon();
        let mut hud = self.hud.bind_mut();
        hud.update_lives_hud(self.current_lives, self.lives);
        hud.update_hp_hud(self.current_health, self.health);
        hud.update_speed_hud(self.current_speed);
        hud.update_damage_hud(self.damage.saturating_add(rust_weapon.bind().get_damage()));
        hud.update_distance_hud(self.distance + rust_weapon.bind().get_distance());
        hud.update_repel_hud(self.repel + rust_weapon.bind().get_repel());
        hud.update_penetrate_hud(self.penetrate + rust_weapon.bind().get_penetrate());
        hud.update_killed_hud(self.kill_boss_count, self.kill_count);
        hud.update_score_hud(self.score);
        hud.update_died_hud(self.died);
        if self.grenade_scenes.is_empty() {
            #[allow(clippy::borrow_interior_mutable_const)]
            self.grenade_scenes.push(&*GRENADE);
        }
        if !self.player_name.is_empty() {
            let name = self.player_name.clone();
            let mut name_label = self.remote_transform2d.get_node_as::<Label>("Name");
            name_label.set_text(&name);
            name_label.show();
        }
        Self::reset_last_score_update();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if RustWorld::is_paused() {
            return;
        }
        if event.is_action_pressed("e") {
            self.chop();
        } else if event.is_action_pressed("r") {
            self.reload();
        } else if event.is_action_released("shift")
            || event.is_action_released("mouse_left")
            || event.is_action_released("mouse_right")
        {
            self.guard();
        } else if event.is_action_pressed("q") || event.is_action_pressed("mouse_middle") {
            self.throw_grenade();
        } else if event.is_action_pressed("1") {
            self.change_weapon(0);
        } else if event.is_action_pressed("2") {
            self.change_weapon(1);
        } else if event.is_action_pressed("3") {
            self.change_weapon(2);
        } else if event.is_action_pressed("4") {
            self.change_weapon(3);
        } else if event.is_action_pressed("5") {
            self.change_weapon(4);
        } else if event.is_action_pressed("6") {
            self.change_weapon(5);
        } else if event.is_action_pressed("7") {
            self.change_weapon(6);
        } else if event.is_action_pressed("8") {
            self.change_weapon(7);
        } else if event.is_action_pressed("9") {
            let mut new_weapon_index = self.current_weapon_index;
            new_weapon_index += 1;
            if new_weapon_index >= self.weapons.get_child_count() {
                new_weapon_index = 0;
            }
            self.change_weapon(new_weapon_index);
        } else if event.is_action_pressed("0") {
            let mut new_weapon_index = self.current_weapon_index;
            new_weapon_index -= 1;
            if new_weapon_index < 0 {
                new_weapon_index = self.weapons.get_child_count() - 1;
            }
            self.change_weapon(new_weapon_index);
        }
    }
}

#[godot_api]
impl RustPlayer {
    pub fn scale(&self) {
        self.base()
            .get_window()
            .unwrap()
            .set_content_scale_factor(scale_rate());
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

    pub fn throw_grenade(&mut self) {
        if self.current_grenade_cooldown > 0.0 {
            if let Some(mut grenade_cooldown_label) = self.create_message() {
                grenade_cooldown_label.bind_mut().show_message(&format!(
                    "GRENADE READY IN {:.1}S",
                    self.current_grenade_cooldown
                ));
            }
            return;
        }
        let direction = self
            .base()
            .get_global_position()
            .direction_to(self.get_mouse_position())
            .normalized();
        let grenade_point = self.grenade_point.get_global_position();
        for grenade_scene in self.grenade_scenes.iter_shared() {
            if let Some(mut grenade) = grenade_scene.try_instantiate_as::<RustGrenade>() {
                grenade.set_global_position(grenade_point);
                let mut gd_mut = grenade.bind_mut();
                gd_mut.set_bullet_point(grenade_point);
                gd_mut.set_final_distance(GRENADE_DISTANCE + self.distance);
                gd_mut.set_final_damage(GRENADE_DAMAGE + self.damage);
                gd_mut.set_final_repel(GRENADE_REPEL + self.repel);
                gd_mut.set_direction(direction);
                drop(gd_mut);
                if let Some(mut parent) = self.base().get_parent() {
                    parent.add_child(&grenade);
                    self.current_grenade_cooldown = self.grenade_cooldown;
                }
            }
        }
    }

    pub fn create_message(&mut self) -> Option<Gd<RustMessage>> {
        #[allow(clippy::borrow_interior_mutable_const)]
        if let Some(mut message_label) = MESSAGE.try_instantiate_as::<RustMessage>() {
            message_label.set_global_position(Self::get_position());
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&message_label);
                    return Some(message_label);
                }
            }
        }
        None
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        self.base().get_canvas_transform().affine_inverse()
            * self
                .base()
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }

    #[func]
    pub fn add_kill_count(&mut self) {
        self.kill_count += 1;
    }

    #[func]
    pub fn add_kill_boss_count(&mut self) {
        self.kill_boss_count += 1;
    }

    #[func]
    pub fn add_score(&mut self, score: u32) {
        self.score += score;
        Self::reset_last_score_update();
    }

    pub fn get_last_score_update() -> u64 {
        LAST_SCORE_UPDATE.load(Ordering::Acquire)
    }

    pub fn reset_last_score_update() {
        LAST_SCORE_UPDATE.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("1970-01-01 00:00:00 UTC was {} seconds ago!")
                .as_secs(),
            Ordering::Release,
        );
    }

    pub fn get_position() -> Vector2 {
        POSITION.load()
    }

    pub fn get() -> Gd<Self> {
        RustWorld::get().get_node_as::<Self>("RustPlayer")
    }
}
