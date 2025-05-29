use crate::common::RustMessage;
use crate::grenade::RustGrenade;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::weapon::RustWeapon;
use crate::world::RustWorld;
use crate::zombie::NEXT_ATTACK_DIRECTION;
use crate::zombie::animation::ZombieAnimation;
use crate::zombie::explode::ZombieExplodeArea;
use crate::{
    BOOMER_ALARM_DISTANCE, BOOMER_DAMAGE, BOOMER_EXPLODE_COUNTDOWN, BOOMER_MOVE_SPEED,
    BOOMER_REPEL, EXPLODE_AUDIOS, GRENADE_ALARM_DISTANCE, GUN_ALARM_DISTANCE, MESSAGE, NO_NOISE,
    PlayerState, SAVE, ZOMBIE_ALARM_TIME, ZOMBIE_MAX_DISTANCE, ZOMBIE_MAX_HEALTH,
    ZOMBIE_PURSUIT_DISTANCE, ZOMBIE_RAMPAGE_TIME, ZOMBIE_REFRESH_BARRIER, ZombieState, random_bool,
    random_direction, random_position,
};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Callable, GString, Vector2, real};
use godot::classes::{
    AnimatedSprite2D, Area2D, AudioStreamPlayer2D, CharacterBody2D, CollisionShape2D, Control,
    GpuParticles2D, ICharacterBody2D, InputEvent, Label, Node, ProgressBar, RemoteTransform2D,
};
use godot::global::godot_error;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::time::{Duration, Instant};

pub mod save;

static NOISE_POSITION: AtomicCell<Vector2> = AtomicCell::new(NO_NOISE);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustBoomer {
    #[export]
    boomer_name: GString,
    #[export]
    invincible: bool,
    #[export]
    moveable: bool,
    #[export]
    rotatable: bool,
    #[export]
    explosive: bool,
    #[export]
    explode_countdown: real,
    #[export]
    health: u32,
    #[export]
    speed: real,
    #[export]
    rampage_time: real,
    #[export]
    alarm_time: real,
    state: ZombieState,
    current_speed: real,
    collision: Vector2,
    current_alarm_time: real,
    last_rotate_time: Instant,
    rotate_cooldown: Duration,
    pursuit_direction: bool,
    current_flash_cooldown: f64,
    hud: OnReady<Gd<RemoteTransform2D>>,
    head_shape2d: OnReady<Gd<CollisionShape2D>>,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    zombie_explode_area: OnReady<Gd<ZombieExplodeArea>>,
    zombie_damage_area: OnReady<Gd<Area2D>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    blood_flash: OnReady<Gd<GpuParticles2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    guard_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    run_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    rampage_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    attack_scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_flash: OnReady<Gd<AnimatedSprite2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustBoomer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            boomer_name: GString::from("BOOMER"),
            invincible: false,
            moveable: true,
            rotatable: true,
            explosive: true,
            explode_countdown: BOOMER_EXPLODE_COUNTDOWN,
            speed: BOOMER_MOVE_SPEED,
            rampage_time: ZOMBIE_RAMPAGE_TIME,
            alarm_time: ZOMBIE_ALARM_TIME,
            health: ZOMBIE_MAX_HEALTH,
            state: ZombieState::Guard,
            current_speed: BOOMER_MOVE_SPEED * 0.75,
            collision: Vector2::ZERO,
            current_alarm_time: 0.0,
            last_rotate_time: Instant::now(),
            rotate_cooldown: Duration::from_secs(8),
            pursuit_direction: random_bool(),
            current_flash_cooldown: 0.0,
            hud: OnReady::from_node("RemoteTransform2D"),
            head_shape2d: OnReady::from_node("HeadShape2D"),
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            zombie_explode_area: OnReady::from_node("ZombieExplodeArea"),
            zombie_damage_area: OnReady::from_node("ZombieDamageArea"),
            hit_audio: OnReady::from_node("HitAudio"),
            blood_flash: OnReady::from_node("GpuParticles2D"),
            guard_audio: OnReady::from_node("GuardAudio"),
            run_audio: OnReady::from_node("RunAudio"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            rampage_audio: OnReady::from_node("RampageAudio"),
            attack_scream_audio: OnReady::from_node("AttackScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            die_flash: OnReady::from_node("DieFlash"),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        if self.hud.is_instance_valid() {
            self.hud.set_global_rotation_degrees(0.0);
        }
        if RustWorld::is_paused() || ZombieState::Dead == self.state {
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
            } else if let Some(noise_position) = RustGrenade::get_noise_position() {
                self.alarmed_by_sound(noise_position, GRENADE_ALARM_DISTANCE)
            } else if let Some(noise_position) = Self::get_noise_position() {
                self.alarmed_by_sound(noise_position, BOOMER_ALARM_DISTANCE)
            } else if let Some(noise_position) = RustWeapon::get_noise_position() {
                self.alarmed_by_sound(noise_position, GUN_ALARM_DISTANCE)
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
                if object.is_class("RustBoomer") || object.is_class("RustBoss") {
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

    fn exit_tree(&mut self) {
        self.die_flash.queue_free();
    }

    fn ready(&mut self) {
        self.die_flash.set_visible(false);
        let gd = self.to_gd();
        self.attack_scream_audio
            .signals()
            .finished()
            .connect_obj(&gd, Self::dying);
        self.die_flash
            .signals()
            .animation_finished()
            .connect_obj(&gd, Self::clean);
        self.last_rotate_time -= self.rotate_cooldown;
        self.guard();
        if !self.boomer_name.is_empty() {
            let name = self.boomer_name.clone();
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
impl RustBoomer {
    pub fn alarmed_by_sound(
        &mut self,
        noise_position: Vector2,
        max_alarm_distance: real,
    ) -> Vector2 {
        let zombie_position = self.base().get_global_position();
        let distance_to_noise = zombie_position.distance_to(noise_position);
        if ZOMBIE_PURSUIT_DISTANCE < distance_to_noise && distance_to_noise < max_alarm_distance {
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
        //面对BOOMER击退力下降25%
        let moved = direction * repel * 0.75;
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
    }

    #[func]
    pub fn dying(&mut self) {
        self.attack_scream_audio.play();
        self.guard();
        self.state = ZombieState::Attack;
        self.get_alarm_progress().set_visible(false);
    }

    #[func]
    pub fn die(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.current_speed = 0.0;
        self.state = ZombieState::Dead;
        if self.die_audio.is_inside_tree() && self.explosive {
            //播放爆炸音效
            #[allow(clippy::borrow_interior_mutable_const)]
            if let Some(audio) = EXPLODE_AUDIOS.pick_random() {
                self.die_audio.set_stream(&audio);
                self.die_audio.play();
                self.die_flash.set_visible(true);
                self.die_flash.set_global_rotation_degrees(0.0);
                self.die_flash.play_ex().name("default").done();
                self.animated_sprite2d.queue_free();
            }
            let position = self.base().get_global_position();
            for mut body in self
                .zombie_damage_area
                .get_overlapping_bodies()
                .iter_shared()
            {
                if !body.is_instance_valid() {
                    continue;
                }
                if body.is_class("RustPlayer") {
                    body.cast::<RustPlayer>()
                        .bind_mut()
                        .on_hit(BOOMER_DAMAGE, position);
                } else if body.is_class("RustZombie")
                    || body.is_class("RustBoss")
                    || body.is_class("RustBoomer")
                {
                    if position != body.get_global_position() {
                        let direction = position.direction_to(body.get_global_position());
                        body.call_deferred(
                            "on_hit",
                            &[
                                BOOMER_DAMAGE.to_variant(),
                                direction.to_variant(),
                                BOOMER_REPEL.to_variant(),
                                position.to_variant(),
                            ],
                        );
                        RustPlayer::get().call_deferred("add_score", &[BOOMER_DAMAGE.to_variant()]);
                    }
                } else if body.is_class("RustGrenade") {
                    // ok
                } else {
                    godot_error!("Boomer hit an unexpected body: {}", body.get_class());
                }
            }
        }
        NOISE_POSITION.store(self.base().get_global_position());
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(3.0) {
                timer.connect(
                    "timeout",
                    &Callable::from_sync_fn("clean_boomer_noise", |_| {
                        NOISE_POSITION.store(NO_NOISE);
                        Ok(().to_variant())
                    }),
                );
            }
        }
        // 释放资源
        self.hud.queue_free();
        self.head_shape2d.queue_free();
        self.collision_shape2d.queue_free();
        self.zombie_explode_area.queue_free();
        self.zombie_damage_area.queue_free();
        self.hit_audio.queue_free();
        self.blood_flash.queue_free();
        self.scream_audio.queue_free();
        self.guard_audio.queue_free();
        self.run_audio.queue_free();
        self.rampage_audio.queue_free();
        self.attack_scream_audio.queue_free();
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
    pub fn clean(&mut self) {
        self.die_audio.set_stream(Gd::null_arg());
        self.die_audio.queue_free();
        // 自动清理尸体
        self.base_mut().queue_free();
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

    pub fn get_noise_position() -> Option<Vector2> {
        let r = NOISE_POSITION.load();
        if NO_NOISE == r { None } else { Some(r) }
    }
}
