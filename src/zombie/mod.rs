use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::animation::ZombieAnimation;
use crate::zombie::attack::{ZombieAttackArea, ZombieDamageArea};
use crate::zombie::hit::ZombieHit;
use crate::{
    PlayerState, ZOMBIE_ALARM_TIME, ZOMBIE_MAX_BODY_COUNT, ZOMBIE_MAX_DISTANCE, ZOMBIE_MAX_HEALTH,
    ZOMBIE_MIN_REFRESH_BATCH, ZOMBIE_MOVE_SPEED, ZOMBIE_PURSUIT_DISTANCE, ZOMBIE_RAMPAGE_TIME,
    ZombieState,
};
use godot::builtin::{Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, CharacterBody2D, CollisionShape2D, GpuParticles2D, ICharacterBody2D,
    PackedScene,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub mod attack;

pub mod animation;

pub mod hit;

static BODY_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustZombie {
    #[export]
    health: u32,
    #[export]
    speed: real,
    #[export]
    rampage_time: real,
    #[export]
    alarm_time: real,
    current_alarm_time: real,
    last_turn_time: Instant,
    turn_cooldown: Duration,
    state: ZombieState,
    current_speed: real,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    zombie_attack_area: OnReady<Gd<ZombieAttackArea>>,
    zombie_damage_area: OnReady<Gd<ZombieDamageArea>>,
    hit_scene: OnReady<Gd<PackedScene>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    blood_flash: OnReady<Gd<GpuParticles2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    guard_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    run_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    rampage_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    attack_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    attack_scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustZombie {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            speed: ZOMBIE_MOVE_SPEED,
            health: ZOMBIE_MAX_HEALTH,
            rampage_time: ZOMBIE_RAMPAGE_TIME,
            alarm_time: ZOMBIE_ALARM_TIME,
            current_alarm_time: 0.0,
            last_turn_time: Instant::now(),
            turn_cooldown: Duration::from_secs(5),
            state: ZombieState::Guard,
            current_speed: ZOMBIE_MOVE_SPEED * 0.2,
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            zombie_attack_area: OnReady::from_node("ZombieAttackArea"),
            zombie_damage_area: OnReady::from_node("ZombieDamageArea"),
            hit_scene: OnReady::from_loaded("res://scenes/zombie_hit.tscn"),
            hit_audio: OnReady::from_node("HitAudio"),
            blood_flash: OnReady::from_node("GpuParticles2D"),
            guard_audio: OnReady::from_node("GuardAudio"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            run_audio: OnReady::from_node("RunAudio"),
            rampage_audio: OnReady::from_node("RampageAudio"),
            attack_audio: OnReady::from_node("AttackAudio"),
            attack_scream_audio: OnReady::from_node("AttackScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            base,
        }
    }

    fn ready(&mut self) {
        self.last_turn_time -= self.turn_cooldown;
        self.animated_sprite2d
            .signals()
            .change_zombie_state()
            .connect_self(ZombieAnimation::on_change_zombie_state);
        self.guard();
    }

    fn process(&mut self, delta: f64) {
        if ZombieState::Dead == self.state || RustWorld::is_paused() {
            if BODY_COUNT.load(Ordering::Acquire) >= ZOMBIE_MAX_BODY_COUNT {
                self.base_mut().queue_free();
                BODY_COUNT.fetch_sub(1, Ordering::Release);
            }
            return;
        }
        if PlayerState::Dead == RustPlayer::get_state() {
            self.move_back();
            return;
        }
        if ZombieState::Attack == self.state {
            return;
        }
        self.rampage_time = (self.rampage_time - delta as real).max(0.0);
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        let distance = zombie_position.distance_to(player_position);
        if distance <= ZOMBIE_PURSUIT_DISTANCE && self.is_face_to_user() {
            self.current_alarm_time =
                (self.current_alarm_time + delta as real).min(self.alarm_time);
        } else {
            self.current_alarm_time = (self.current_alarm_time - delta as real).max(0.0);
        }
        let to_player_dir = zombie_position.direction_to(player_position).normalized();
        let mut character_body2d = self.base.to_gd();
        //僵尸之间的体积碰撞检测
        for i in 0..character_body2d.get_slide_collision_count() {
            if let Some(collision) = character_body2d.get_slide_collision(i) {
                // 发出排斥力的方向
                let from = collision.get_normal();
                if let Some(object) = collision.get_collider() {
                    if object.is_class("RustZombie") {
                        // 受到排斥的僵尸
                        let mut to_zombie = object.cast::<Self>();
                        if ZombieState::Run == to_zombie.bind().state
                            || ZombieState::Rampage == to_zombie.bind().state
                        {
                            continue;
                        }
                        // 给其他僵尸让开位置
                        let dir = from.orthogonal();
                        let speed = to_zombie.bind().current_speed;
                        to_zombie.look_at(player_position);
                        to_zombie.set_velocity(dir * speed);
                        to_zombie.move_and_slide();
                        to_zombie.look_at(player_position);
                        to_zombie.set_velocity((-from) * speed);
                        to_zombie.move_and_slide();
                    }
                }
            }
        }
        if distance >= ZOMBIE_MAX_DISTANCE {
            //解决刷新僵尸导致的体积碰撞问题
            self.flash();
        } else if self.is_alarmed() || RustLevel::is_rampage() {
            // 跑向玩家
            self.rampage();
            self.base_mut().look_at(player_position);
            character_body2d.set_velocity(to_player_dir * self.current_speed);
        } else {
            // 无目的移动
            if self.is_rampage_run() {
                self.run();
            } else {
                self.guard();
            }
            let now = Instant::now();
            if now.duration_since(self.last_turn_time) >= self.turn_cooldown {
                let direction = Self::random_direction();
                character_body2d.look_at(zombie_position + direction);
                character_body2d.set_velocity(direction * self.current_speed);
                self.last_turn_time = now;
            }
        }
        character_body2d.move_and_slide();
    }
}

#[godot_api]
impl RustZombie {
    #[func]
    pub fn on_hit(&mut self, hit_val: i64, direction: Vector2, repel: real, hit_position: Vector2) {
        let zombie_position = self.base().get_global_position();
        if let Some(mut hit_label) = self.hit_scene.try_instantiate_as::<ZombieHit>() {
            hit_label.set_global_position(zombie_position);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&hit_label);
                    hit_label.bind_mut().show_hit_value(hit_val);
                }
            }
        }
        let health = self.health;
        self.health = if hit_val > 0 {
            health.saturating_sub(hit_val as u32)
        } else {
            health.saturating_add(-hit_val as u32)
        };
        let speed = self.current_speed;
        let moved = direction * repel;
        let new_position = zombie_position + moved;
        let mut base_mut = self.base_mut();
        base_mut.look_at(zombie_position - direction);
        base_mut.set_velocity(-direction * speed);
        //僵尸被击退
        base_mut.set_global_position(new_position);
        //僵尸往被攻击的方向移动
        base_mut.move_and_slide();
        drop(base_mut);
        if 0 != self.health {
            self.hit(direction, hit_position);
        } else {
            self.die();
        }
    }

    pub fn random_direction() -> Vector2 {
        let mut rng = rand::thread_rng();
        Vector2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)).normalized()
    }

    fn notify_animation(&mut self) {
        self.animated_sprite2d
            .signals()
            .change_zombie_state()
            .emit(self.state);
    }

    #[func]
    pub fn guard(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.2;
        self.state = ZombieState::Guard;
        if !self.guard_audio.is_playing() && self.guard_audio.is_inside_tree() {
            if RustLevel::get_live_count() >= ZOMBIE_MIN_REFRESH_BATCH {
                self.guard_audio.set_volume_db(-30.0);
            } else {
                self.guard_audio.set_volume_db(-20.0);
            }
            self.guard_audio.play();
        }
        self.notify_animation();
    }

    pub fn run(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.75;
        self.state = ZombieState::Run;
        if !self.run_audio.is_playing() && self.run_audio.is_inside_tree() {
            self.run_audio.play();
        }
        self.notify_animation();
    }

    pub fn hit(&mut self, direction: Vector2, hit_position: Vector2) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.1;
        self.state = ZombieState::Hit;
        self.blood_flash.set_global_position(hit_position);
        self.blood_flash.look_at(hit_position - direction);
        self.blood_flash.restart();
        if self.hit_audio.is_inside_tree() {
            self.hit_audio.play();
        }
        if self.scream_audio.is_inside_tree() {
            self.scream_audio.play();
        }
        self.notify_animation();
    }

    pub fn rampage(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("run").done();
        self.current_speed = self.speed * 1.75;
        self.state = ZombieState::Rampage;
        if !self.rampage_audio.is_playing() && self.rampage_audio.is_inside_tree() {
            if RustLevel::get_live_count() >= ZOMBIE_MIN_REFRESH_BATCH {
                self.rampage_audio.set_volume_db(-40.0);
            } else {
                self.rampage_audio.set_volume_db(-10.0);
            }
            self.rampage_audio.play();
        }
        self.notify_animation();
    }

    pub fn attack(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("attack").done();
        self.current_speed = self.speed * 0.5;
        self.state = ZombieState::Attack;
        if self.attack_audio.is_inside_tree() {
            self.attack_audio.play();
        }
        if !self.attack_scream_audio.is_playing() && self.attack_scream_audio.is_inside_tree() {
            self.attack_scream_audio.play();
        }
        self.notify_animation();
    }

    pub fn die(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("die").done();
        self.current_speed = 0.0;
        self.state = ZombieState::Dead;
        if self.die_audio.is_inside_tree() {
            self.die_audio.play();
        }
        // 释放资源
        self.base_mut().set_z_index(0);
        self.collision_shape2d.queue_free();
        self.zombie_attack_area.queue_free();
        self.zombie_damage_area.queue_free();
        self.notify_animation();
        // 击杀僵尸确认
        self.base()
            .get_tree()
            .unwrap()
            .get_root()
            .unwrap()
            .get_node_as::<RustWorld>("RustWorld")
            .get_node_as::<RustLevel>("RustLevel")
            .bind_mut()
            .kill_confirmed();
        BODY_COUNT.fetch_add(1, Ordering::Release);
    }

    pub fn flash(&mut self) {
        let player_position = RustPlayer::get_position();
        self.base_mut().look_at(-player_position);
        self.base_mut().set_global_position(
            player_position
                + Vector2::new(Self::random_half_position(), Self::random_half_position()),
        );
    }

    fn random_half_position() -> real {
        let mut rng = rand::thread_rng();
        if rng.gen_range(-1.0..1.0) >= 0.0 {
            rng.gen_range(900.0..1100.0)
        } else {
            rng.gen_range(-1100.0..-900.0)
        }
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
        zombie.set_velocity(from_player_dir * speed);
        zombie.move_and_slide();
    }

    // 看到玩家不会马上狂暴，而是累计时间条，类似刺客信条
    pub fn is_alarmed(&self) -> bool {
        self.current_alarm_time >= self.alarm_time
    }

    pub fn is_rampage_run(&self) -> bool {
        if PlayerState::Dead == RustPlayer::get_state() {
            return false;
        }
        self.rampage_time <= 0.0
    }

    pub fn get_distance(&self) -> real {
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        zombie_position.distance_to(player_position)
    }

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
}
