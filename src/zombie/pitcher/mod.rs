use crate::common::RustMessage;
use crate::grenade::RustGrenade;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::weapon::RustWeapon;
use crate::world::RustWorld;
use crate::zombie::BODY_COUNT;
use crate::zombie::animation::ZombieAnimation;
use crate::zombie::boomer::RustBoomer;
use crate::zombie::pitch::ZombiePitchArea;
use crate::{
    BOOMER_ALARM_DISTANCE, GRENADE_ALARM_DISTANCE, GUN_ALARM_DISTANCE, MESSAGE,
    PITCHER_ALARM_DISTANCE, PITCHER_ATTACK_DISTANCE, PITCHER_DAMAGE, PITCHER_GRENADE_COUNTDOWN,
    PITCHER_MOVE_SPEED, PITCHER_PURSUIT_DISTANCE, PITCHER_REPEL, PLAYER_ALARM_DISTANCE,
    PlayerState, ZOMBIE_ALARM_TIME, ZOMBIE_GRENADE_DISTANCE, ZOMBIE_MAX_BODY_COUNT,
    ZOMBIE_MAX_DISTANCE, ZOMBIE_MAX_HEALTH, ZOMBIE_MIN_TRACK_DISTANCE, ZOMBIE_RAMPAGE_TIME,
    ZOMBIE_ROTATE_COOLDOWN, ZombieState, not_normal_zombie, random_bool, random_direction,
    random_position,
};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Array, GString, Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, CharacterBody2D, CollisionShape2D, Control, GpuParticles2D,
    ICharacterBody2D, InputEvent, Label, Node2D, PackedScene, ProgressBar, RemoteTransform2D,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use godot::tools::load;
use std::sync::LazyLock;
use std::sync::atomic::Ordering;

pub mod state;

pub mod save;

static NEXT_ATTACK_DIRECTION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

#[allow(clippy::declare_interior_mutable_const)]
const ZOMBIE_GRENADE: LazyLock<Gd<PackedScene>> =
    LazyLock::new(|| load("res://scenes/grenades/zombie_grenade.tscn"));

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPitcher {
    #[export]
    pitcher_name: GString,
    #[export]
    invincible: bool,
    #[export]
    moveable: bool,
    #[export]
    rotatable: bool,
    #[export]
    rotate_cooldown: real,
    #[export]
    attackable: bool,
    #[export]
    grenade_cooldown: real,
    // 手雷类型
    #[export]
    grenade_scenes: Array<Gd<PackedScene>>,
    #[export]
    health: u32,
    #[export]
    speed: real,
    #[export]
    rampage_time: real,
    #[export]
    alarm_time: real,
    attacking: bool,
    current_grenade_cooldown: real,
    current_alarm_time: real,
    current_rotate_cooldown: real,
    state: ZombieState,
    current_speed: real,
    collision: Vector2,
    pursuit_direction: bool,
    current_flash_cooldown: f64,
    hud: OnReady<Gd<RemoteTransform2D>>,
    head_shape2d: OnReady<Gd<CollisionShape2D>>,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    zombie_pitch_area: OnReady<Gd<ZombiePitchArea>>,
    born_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    blood_flash: OnReady<Gd<GpuParticles2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    guard_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    run_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    rampage_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    attack_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    attack_scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    grenade_point: OnReady<Gd<Node2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPitcher {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            pitcher_name: GString::new(),
            invincible: false,
            moveable: true,
            rotatable: true,
            rotate_cooldown: ZOMBIE_ROTATE_COOLDOWN,
            attackable: true,
            grenade_cooldown: PITCHER_GRENADE_COUNTDOWN,
            grenade_scenes: Array::new(),
            speed: PITCHER_MOVE_SPEED,
            health: ZOMBIE_MAX_HEALTH,
            rampage_time: ZOMBIE_RAMPAGE_TIME,
            alarm_time: ZOMBIE_ALARM_TIME,
            attacking: false,
            current_grenade_cooldown: 0.0,
            current_alarm_time: 0.0,
            current_rotate_cooldown: 0.0,
            state: ZombieState::Guard,
            current_speed: PITCHER_MOVE_SPEED * 0.2,
            collision: Vector2::ZERO,
            pursuit_direction: random_bool(),
            current_flash_cooldown: 0.0,
            hud: OnReady::from_node("RemoteTransform2D"),
            head_shape2d: OnReady::from_node("HeadShape2D"),
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            zombie_pitch_area: OnReady::from_node("ZombiePitchArea"),
            born_audio: OnReady::from_node("BornAudio"),
            hit_audio: OnReady::from_node("HitAudio"),
            blood_flash: OnReady::from_node("GpuParticles2D"),
            guard_audio: OnReady::from_node("GuardAudio"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            run_audio: OnReady::from_node("RunAudio"),
            rampage_audio: OnReady::from_node("RampageAudio"),
            attack_audio: OnReady::from_node("AttackAudio"),
            attack_scream_audio: OnReady::from_node("AttackScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            grenade_point: OnReady::from_node("GrenadePoint"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        if self.hud.is_instance_valid() {
            self.hud.set_global_rotation_degrees(0.0);
        }
        let player_state = RustPlayer::get_state();
        if RustWorld::is_paused() || PlayerState::Impact == player_state {
            return;
        }
        if ZombieState::Dead == self.state {
            if BODY_COUNT.load(Ordering::Acquire) >= ZOMBIE_MAX_BODY_COUNT {
                self.clean_body();
            }
            return;
        }
        if PlayerState::Dead == player_state {
            self.move_back();
            return;
        }
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        let distance = zombie_position.distance_to(player_position);
        self.current_grenade_cooldown -= delta as real;
        if self.attacking || self.is_face_to_user() && distance <= PITCHER_ATTACK_DISTANCE {
            self.throw_grenade();
            return;
        }
        self.current_rotate_cooldown -= delta as real;
        self.current_flash_cooldown -= delta;
        self.rampage_time = (self.rampage_time - delta as real).max(0.0);
        if distance >= ZOMBIE_MAX_DISTANCE {
            //解决刷新僵尸导致的体积碰撞问题
            self.flash();
            return;
        }
        self.update_alarm_progress_hud(delta);
        let to_player_dir = zombie_position.direction_to(player_position).normalized();
        let real_to_player_dir = if Vector2::ZERO != self.collision {
            self.collision
        } else {
            to_player_dir
        };
        let mut character_body2d = self.base.to_gd();
        let velocity = if distance > PITCHER_ATTACK_DISTANCE
            && (self.is_alarmed() || RustLevel::is_rampage())
        {
            // 跑向玩家
            self.rampage();
            self.base_mut().look_at(player_position);
            real_to_player_dir * self.current_speed
        } else {
            if self.is_rampage_run() {
                self.run();
            } else {
                self.guard();
            }
            if distance <= PITCHER_PURSUIT_DISTANCE
                && self.current_alarm_time > 0.0
                && self.is_face_to_user()
            {
                // 向玩家移动，并累计警戒条
                self.base_mut().look_at(player_position);
                real_to_player_dir * self.current_speed
            } else if let Some(noise_position) = RustGrenade::get_noise_position() {
                self.alarmed_by_sound(noise_position, GRENADE_ALARM_DISTANCE)
            } else if let Some(noise_position) = RustBoomer::get_noise_position() {
                self.alarmed_by_sound(noise_position, BOOMER_ALARM_DISTANCE)
            } else if let Some(noise_position) = RustGrenade::get_zombie_noise_position() {
                self.alarmed_by_sound(noise_position, PITCHER_ALARM_DISTANCE)
            } else if let Some(noise_position) = RustWeapon::get_noise_position() {
                self.alarmed_by_sound(noise_position, GUN_ALARM_DISTANCE)
            } else if let Some(noise_position) = RustPlayer::get_noise_position() {
                self.alarmed_by_sound(noise_position, PLAYER_ALARM_DISTANCE)
            } else if self.rotatable && self.current_rotate_cooldown <= 0.0 {
                // 无目的移动
                let direction = random_direction();
                character_body2d.look_at(zombie_position + direction);
                self.current_rotate_cooldown = self.rotate_cooldown;
                direction * self.current_speed
            } else {
                self.guard();
                self.get_current_direction() * self.current_speed
            }
        };
        if !self.moveable {
            return;
        }
        //撞到僵尸了
        self.collision = Vector2::ZERO;
        if let Some(collision) = character_body2d.move_and_collide(velocity) {
            // 发出排斥力的方向
            let from = collision.get_normal();
            if let Some(object) = collision.get_collider() {
                if not_normal_zombie(&object) {
                    let dir = NEXT_ATTACK_DIRECTION.load();
                    let move_angle = to_player_dir.angle_to(dir).to_degrees();
                    self.collision =
                        if (0.0..=120.0).contains(&move_angle) || dir.x.abs() >= dir.y.abs() {
                            from.orthogonal()
                        } else if (-120.0..0.0).contains(&move_angle) || dir.x.abs() < dir.y.abs() {
                            -from.orthogonal()
                        } else if self.pursuit_direction {
                            from.orthogonal()
                        } else {
                            -from.orthogonal()
                        };
                }
            }
        }
    }

    fn ready(&mut self) {
        let gd = self.to_gd();
        self.die_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::clean_audio);
        self.current_rotate_cooldown = self.rotate_cooldown;
        self.born_audio.play();
        self.guard();
        if self.grenade_scenes.is_empty() {
            #[allow(clippy::borrow_interior_mutable_const)]
            self.grenade_scenes.push(&*ZOMBIE_GRENADE);
        }
        if !self.pitcher_name.is_empty() {
            let name = self.pitcher_name.clone();
            let mut name_label = self.hud.get_node_as::<Label>("Name");
            name_label.set_text(&name);
            name_label.show();
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if cfg!(feature = "develop") && event.is_action_pressed("k") {
            self.die();
        }
    }
}

#[godot_api]
impl RustPitcher {
    pub fn alarmed_by_sound(
        &mut self,
        noise_position: Vector2,
        max_alarm_distance: real,
    ) -> Vector2 {
        let zombie_position = self.base().get_global_position();
        let distance_to_noise = zombie_position.distance_to(noise_position);
        if ZOMBIE_MIN_TRACK_DISTANCE < distance_to_noise && distance_to_noise < max_alarm_distance {
            // 向噪音位置移动
            self.base_mut().look_at(noise_position);
            self.current_alarm_time = self.alarm_time;
            zombie_position.direction_to(noise_position).normalized() * self.current_speed
        } else {
            self.guard();
            self.get_current_direction() * self.current_speed
        }
    }

    #[func]
    pub fn on_hit(&mut self, hit_val: i64, direction: Vector2, repel: real, hit_position: Vector2) {
        let zombie_position = self.base().get_global_position();
        #[allow(clippy::borrow_interior_mutable_const)]
        if let Some(mut hit_label) = MESSAGE.try_instantiate_as::<RustMessage>() {
            hit_label.set_global_position(zombie_position);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&hit_label);
                    hit_label.bind_mut().show_hit_value(hit_val);
                }
            }
        }
        if !self.invincible {
            let health = self.health;
            self.health = if hit_val > 0 {
                health.saturating_sub(hit_val as u32)
            } else {
                health.saturating_add(-hit_val as u32)
            };
        }
        self.current_alarm_time = self.alarm_time;
        let speed = self.current_speed;
        let moved = direction * repel;
        let new_position = zombie_position + moved;
        let mut base_mut = self.base_mut();
        base_mut.look_at(zombie_position - direction);
        //僵尸被击退
        base_mut.set_global_position(new_position);
        //僵尸往被攻击的方向移动
        base_mut.move_and_collide(-direction * speed);
        drop(base_mut);
        if 0 != self.health {
            self.hit(direction, hit_position);
        } else {
            self.die();
        }
    }

    pub fn flash(&mut self) {
        let player_position = RustPlayer::get_position();
        self.base_mut().look_at(-player_position);
        self.base_mut()
            .set_global_position(player_position + random_position(900.0, 1100.0));
    }

    #[func]
    pub fn throw_grenade(&mut self) {
        let player_position = RustPlayer::get_position();
        self.base_mut().look_at(player_position);
        if self.current_grenade_cooldown > 0.0 {
            return;
        }
        let direction = self
            .base()
            .get_global_position()
            .direction_to(player_position)
            .normalized();
        let grenade_point = self.grenade_point.get_global_position();
        for grenade_scene in self.grenade_scenes.iter_shared() {
            if let Some(mut grenade) = grenade_scene.try_instantiate_as::<RustGrenade>() {
                grenade.set_global_position(grenade_point);
                let mut gd_mut = grenade.bind_mut();
                gd_mut.set_bullet_point(grenade_point);
                gd_mut.set_final_distance(ZOMBIE_GRENADE_DISTANCE);
                gd_mut.set_final_damage(PITCHER_DAMAGE);
                gd_mut.set_final_repel(PITCHER_REPEL);
                gd_mut.set_direction(direction);
                drop(gd_mut);
                if let Some(mut parent) = RustPlayer::get().get_parent() {
                    parent.add_child(&grenade);
                    self.current_grenade_cooldown = self.grenade_cooldown;
                }
            }
        }
    }

    #[func]
    pub fn clean_body(&mut self) {
        self.grenade_scenes.clear();
        self.base_mut().queue_free();
        BODY_COUNT.fetch_sub(1, Ordering::Release);
    }

    #[func]
    pub fn clean_audio(&mut self) {
        self.die_audio.queue_free();
    }

    pub fn move_back(&mut self) {
        //僵尸往玩家相反的方向移动一段距离
        self.guard();
        let zombie_position = self.base().get_global_position();
        let from_player_dir = RustPlayer::get_position()
            .direction_to(zombie_position)
            .normalized();
        let speed = self.current_speed;
        let mut zombie = self.base_mut();
        zombie.look_at(zombie_position + from_player_dir);
        zombie.move_and_collide(from_player_dir * speed);
    }

    // 看到玩家不会马上狂暴，而是累计时间条，类似刺客信条
    pub fn is_alarmed(&self) -> bool {
        self.current_alarm_time >= self.alarm_time
    }

    pub fn update_alarm_progress_hud(&mut self, delta: f64) {
        if !self.hud.is_instance_valid() {
            return;
        }
        let mut alarm_progress = self.hud.get_node_as::<Control>("AlarmProgress");
        if 0.0 == self.current_alarm_time || RustLevel::is_rampage() {
            alarm_progress.set_visible(false);
        } else {
            alarm_progress.set_visible(true);
        }
        if self.get_to_player_distance() <= PITCHER_PURSUIT_DISTANCE && self.is_face_to_user() {
            self.current_alarm_time =
                (self.current_alarm_time + delta as real).min(self.alarm_time);
        } else {
            self.current_alarm_time = (self.current_alarm_time - delta as real).max(0.0);
        }
        let progress = if 0.0 == self.current_alarm_time {
            100.0
        } else {
            (self.current_alarm_time / self.alarm_time) as f64 * 100.0
        };
        alarm_progress
            .get_node_as::<ProgressBar>("ProgressBar")
            .set_value_no_signal(progress);
    }

    pub fn is_rampage_run(&self) -> bool {
        if PlayerState::Dead == RustPlayer::get_state() {
            return false;
        }
        self.rampage_time <= 0.0
    }

    pub fn get_to_player_distance(&self) -> real {
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        zombie_position.distance_to(player_position)
    }

    #[func]
    pub fn get_current_direction(&self) -> Vector2 {
        let rotation = self.base().get_rotation();
        Vector2::new(rotation.cos(), rotation.sin())
    }

    pub fn is_face_to_user(&self) -> bool {
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        let to_player_dir = zombie_position.direction_to(player_position).normalized();
        let angle = self
            .get_current_direction()
            .angle_to(to_player_dir)
            .to_degrees();
        (-60.0..=60.0).contains(&angle)
    }

    pub fn set_attacking(&mut self, attacking: bool) {
        self.attacking = attacking;
    }
}
