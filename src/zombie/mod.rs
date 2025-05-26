use crate::common::RustMessage;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::animation::ZombieAnimation;
use crate::zombie::attack::{ZombieAttackArea, ZombieDamageArea};
use crate::{
    MESSAGE, PlayerState, ZOMBIE_ALARM_TIME, ZOMBIE_DAMAGE, ZOMBIE_MAX_BODY_COUNT,
    ZOMBIE_MAX_DISTANCE, ZOMBIE_MAX_HEALTH, ZOMBIE_MOVE_SPEED, ZOMBIE_PURSUIT_DISTANCE,
    ZOMBIE_RAMPAGE_TIME, ZOMBIE_REFRESH_BARRIER, ZOMBIE_SKIP_FRAME, ZombieState, random_bool,
    random_direction, random_position,
};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{GString, Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, CharacterBody2D, CollisionShape2D, Control, GpuParticles2D,
    ICharacterBody2D, InputEvent, Label, Node, ProgressBar, RemoteTransform2D,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub mod attack;

pub mod bump;

pub mod explode;

pub mod animation;

pub mod boomer;

pub mod boss;

static BODY_COUNT: AtomicU32 = AtomicU32::new(0);

static NEXT_ATTACK_DIRECTION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustZombie {
    #[export]
    zombie_name: GString,
    #[export]
    invincible: bool,
    #[export]
    moveable: bool,
    #[export]
    rotatable: bool,
    #[export]
    attackable: bool,
    #[export]
    health: u32,
    #[export]
    speed: real,
    #[export]
    rampage_time: real,
    #[export]
    alarm_time: real,
    current_alarm_time: real,
    last_rotate_time: Instant,
    rotate_cooldown: Duration,
    state: ZombieState,
    current_speed: real,
    hurt_frames: Vec<i32>,
    collision: Vector2,
    frame_counter: u128,
    pursuit_direction: bool,
    current_flash_cooldown: f64,
    hud: OnReady<Gd<RemoteTransform2D>>,
    head_shape2d: OnReady<Gd<CollisionShape2D>>,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    zombie_attack_area: OnReady<Gd<ZombieAttackArea>>,
    zombie_damage_area: OnReady<Gd<ZombieDamageArea>>,
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
            zombie_name: GString::new(),
            invincible: false,
            moveable: true,
            rotatable: true,
            attackable: true,
            speed: ZOMBIE_MOVE_SPEED,
            health: ZOMBIE_MAX_HEALTH,
            rampage_time: ZOMBIE_RAMPAGE_TIME,
            alarm_time: ZOMBIE_ALARM_TIME,
            current_alarm_time: 0.0,
            last_rotate_time: Instant::now(),
            rotate_cooldown: Duration::from_secs(8),
            state: ZombieState::Guard,
            current_speed: ZOMBIE_MOVE_SPEED * 0.2,
            hurt_frames: vec![2, 3, 4, 5],
            collision: Vector2::ZERO,
            frame_counter: 0,
            pursuit_direction: random_bool(),
            current_flash_cooldown: 0.0,
            hud: OnReady::from_node("RemoteTransform2D"),
            head_shape2d: OnReady::from_node("HeadShape2D"),
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            zombie_attack_area: OnReady::from_node("ZombieAttackArea"),
            zombie_damage_area: OnReady::from_node("ZombieDamageArea"),
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

    fn process(&mut self, delta: f64) {
        if self.hud.is_instance_valid() {
            self.hud.set_global_rotation_degrees(0.0);
        }
        self.frame_counter = self.frame_counter.wrapping_add(1);
        if RustWorld::is_paused() || 0 == self.frame_counter % ZOMBIE_SKIP_FRAME {
            return;
        }
        if ZombieState::Dead == self.state {
            if BODY_COUNT.load(Ordering::Acquire) >= ZOMBIE_MAX_BODY_COUNT {
                self.clean_body();
            }
            return;
        }
        let player_state = RustPlayer::get_state();
        if PlayerState::Dead == player_state {
            self.move_back();
            return;
        }
        if ZombieState::Attack == self.state || PlayerState::Impact == player_state {
            return;
        }
        self.current_flash_cooldown -= delta;
        self.rampage_time = (self.rampage_time - delta as real).max(0.0);
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        let distance = zombie_position.distance_to(player_position);
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
        let velocity = if self.is_alarmed() || RustLevel::is_rampage() {
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
            let now = Instant::now();
            if distance <= ZOMBIE_PURSUIT_DISTANCE
                && self.current_alarm_time > 0.0
                && self.is_face_to_user()
            {
                // 向玩家移动，并累计警戒条
                self.base_mut().look_at(player_position);
                real_to_player_dir * self.current_speed
            } else if self.rotatable
                && now.duration_since(self.last_rotate_time) >= self.rotate_cooldown
            {
                // 无目的移动
                let direction = random_direction();
                character_body2d.look_at(zombie_position + direction);
                self.last_rotate_time = now;
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
                if object.is_class("RustZombie")
                    || object.is_class("RustBoomer")
                    || object.is_class("RustBoss")
                {
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
                        }
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
        self.last_rotate_time -= self.rotate_cooldown;
        let mut animated_sprite2d = self.animated_sprite2d.bind_mut();
        animated_sprite2d.set_hurt_frames(self.hurt_frames.clone());
        animated_sprite2d.set_damage(ZOMBIE_DAMAGE);
        drop(animated_sprite2d);
        self.guard();
        if !self.zombie_name.is_empty() {
            let name = self.zombie_name.clone();
            let mut name_label = self.hud.get_node_as::<Label>("Name");
            name_label.set_text(&name);
            name_label.show();
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("k") {
            self.die();
        }
    }
}

#[godot_api]
impl RustZombie {
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
            if RustLevel::get_live_count() >= ZOMBIE_REFRESH_BARRIER {
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
        self.current_speed = self.speed * 1.35;
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
        if self.current_flash_cooldown <= 0.0 {
            self.blood_flash.set_global_position(hit_position);
            self.blood_flash.look_at(hit_position - direction);
            self.blood_flash.restart();
            self.current_flash_cooldown = self.blood_flash.get_lifetime() * 0.25;
        }
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
        self.current_speed = self.speed * 1.6;
        self.state = ZombieState::Rampage;
        if !self.rampage_audio.is_playing() && self.rampage_audio.is_inside_tree() {
            let live_count = RustLevel::get_live_count();
            if live_count >= ZOMBIE_REFRESH_BARRIER {
                self.rampage_audio.set_volume_db(-40.0);
            } else if live_count >= ZOMBIE_REFRESH_BARRIER / 2 {
                self.rampage_audio.set_volume_db(-25.0);
            } else {
                self.rampage_audio.set_volume_db(-12.0);
            }
            self.rampage_audio.play();
        }
        self.notify_animation();
    }

    pub fn attack(&mut self) {
        if ZombieState::Dead == self.state || !self.attackable {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("attack").done();
        self.current_speed = self.speed * 0.5;
        self.state = ZombieState::Attack;
        let direction = NEXT_ATTACK_DIRECTION.load()
            + self
                .base()
                .get_global_position()
                .direction_to(RustPlayer::get_position());
        NEXT_ATTACK_DIRECTION.store(direction.normalized());
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
        self.hud.queue_free();
        self.head_shape2d.queue_free();
        self.collision_shape2d.queue_free();
        self.zombie_attack_area.queue_free();
        self.zombie_damage_area.queue_free();
        self.hit_audio.queue_free();
        self.blood_flash.queue_free();
        self.scream_audio.queue_free();
        self.guard_audio.queue_free();
        self.run_audio.queue_free();
        self.rampage_audio.queue_free();
        self.attack_audio.queue_free();
        self.attack_scream_audio.queue_free();
        self.notify_animation();
        // 45S后自动清理尸体
        BODY_COUNT.fetch_add(1, Ordering::Release);
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(45.0) {
                timer.connect("timeout", &self.base().callable("clean_body"));
            }
        }
        // 击杀僵尸确认
        self.base()
            .get_tree()
            .unwrap()
            .get_root()
            .unwrap()
            .get_node_as::<Node>("RustWorld")
            .get_node_as::<RustLevel>("RustLevel")
            .bind_mut()
            .kill_confirmed();
    }

    #[func]
    pub fn clean_body(&mut self) {
        self.base_mut().queue_free();
        BODY_COUNT.fetch_sub(1, Ordering::Release);
    }

    #[func]
    pub fn clean_audio(&mut self) {
        self.die_audio.queue_free();
    }

    pub fn flash(&mut self) {
        let player_position = RustPlayer::get_position();
        self.base_mut().look_at(-player_position);
        self.base_mut()
            .set_global_position(player_position + random_position(900.0, 1100.0));
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
        let mut alarm_progress = self.get_alarm_progress();
        if 0.0 == self.current_alarm_time || RustLevel::is_rampage() {
            alarm_progress.set_visible(false);
        } else {
            alarm_progress.set_visible(true);
        }
        if self.get_to_player_distance() <= ZOMBIE_PURSUIT_DISTANCE && self.is_face_to_user() {
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

    pub fn get_alarm_progress(&self) -> Gd<Control> {
        self.hud.get_node_as::<Control>("AlarmProgress")
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

    pub fn get_current_speed(&self) -> real {
        self.current_speed
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
