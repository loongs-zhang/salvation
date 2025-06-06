use crate::common::RustMessage;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::NEXT_ATTACK_DIRECTION;
use crate::zombie::animation::ZombieAnimation;
use crate::zombie::attack::{ZombieAttackArea, ZombieDamageArea};
use crate::zombie::bump::BossBumpArea;
use crate::{
    BOSS_BUMP_DISTANCE, BOSS_DAMAGE, BOSS_MAX_BODY_COUNT, BOSS_MAX_HEALTH, BOSS_MOVE_SPEED,
    MESSAGE, PlayerState, ZOMBIE_MAX_DISTANCE, ZombieState, is_boss, not_boss, random_bool,
    random_position,
};
use godot::builtin::{GString, Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, CharacterBody2D, CollisionShape2D, Control, GpuParticles2D,
    ICharacterBody2D, InputEvent, KinematicCollision2D, Label, PhysicsBody2D, ProgressBar,
    RemoteTransform2D,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub mod state;

pub mod save;

static BODY_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustBoss {
    #[export]
    boss_name: GString,
    #[export]
    invincible: bool,
    #[export]
    moveable: bool,
    #[export]
    attackable: bool,
    #[export]
    collidable: bool,
    #[export]
    health: u32,
    #[export]
    speed: real,
    state: ZombieState,
    current_speed: real,
    hurt_frames: Vec<i32>,
    collision: Vector2,
    pursuit_direction: bool,
    last_player_position: Vector2,
    last_record_time: Instant,
    record_cooldown: Duration,
    hud: OnReady<Gd<RemoteTransform2D>>,
    head_shape2d: OnReady<Gd<CollisionShape2D>>,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    zombie_attack_area: OnReady<Gd<ZombieAttackArea>>,
    zombie_damage_area: OnReady<Gd<ZombieDamageArea>>,
    bump_damage_area: OnReady<Gd<BossBumpArea>>,
    born_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    blood_flash: OnReady<Gd<GpuParticles2D>>,
    scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    guard_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    bump_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    attack_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    attack_scream_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    die_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustBoss {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            boss_name: GString::new(),
            invincible: false,
            moveable: true,
            attackable: true,
            collidable: true,
            speed: BOSS_MOVE_SPEED,
            health: BOSS_MAX_HEALTH,
            state: ZombieState::Guard,
            current_speed: BOSS_MOVE_SPEED * 0.75,
            // hurt_frames: vec![26, 27, 28, 29, 30],
            hurt_frames: vec![2, 3, 4, 5],
            collision: Vector2::ZERO,
            pursuit_direction: random_bool(),
            last_player_position: Vector2::ZERO,
            last_record_time: Instant::now(),
            record_cooldown: Duration::from_secs(3),
            hud: OnReady::from_node("RemoteTransform2D"),
            head_shape2d: OnReady::from_node("HeadShape2D"),
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            zombie_attack_area: OnReady::from_node("ZombieAttackArea"),
            zombie_damage_area: OnReady::from_node("ZombieDamageArea"),
            bump_damage_area: OnReady::from_node("BossBumpArea"),
            born_audio: OnReady::from_node("BornAudio"),
            hit_audio: OnReady::from_node("HitAudio"),
            blood_flash: OnReady::from_node("GpuParticles2D"),
            guard_audio: OnReady::from_node("GuardAudio"),
            scream_audio: OnReady::from_node("ScreamAudio"),
            bump_audio: OnReady::from_node("BumpAudio"),
            attack_audio: OnReady::from_node("AttackAudio"),
            attack_scream_audio: OnReady::from_node("AttackScreamAudio"),
            die_audio: OnReady::from_node("DieAudio"),
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        if self.hud.is_instance_valid() {
            self.hud.set_global_rotation_degrees(0.0);
        }
        if RustWorld::is_paused() {
            return;
        }
        if ZombieState::Dead == self.state {
            if BODY_COUNT.load(Ordering::Acquire) >= BOSS_MAX_BODY_COUNT {
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
        let player_position = RustPlayer::get_position();
        let now = Instant::now();
        if now.duration_since(self.last_record_time) >= self.record_cooldown {
            self.last_player_position = player_position;
            self.last_record_time = now;
        }
        let zombie_position = self.base().get_global_position();
        let distance = zombie_position.distance_to(player_position);
        if distance >= ZOMBIE_MAX_DISTANCE {
            //解决刷新僵尸导致的体积碰撞问题
            self.flash();
            return;
        }
        self.update_hp_progress_hud();
        let to_player_dir = zombie_position.direction_to(player_position).normalized();
        let velocity = if distance >= BOSS_BUMP_DISTANCE {
            // 走向玩家
            self.guard();
            self.base_mut().look_at(player_position);
            to_player_dir * self.current_speed
        } else {
            // 冲撞玩家
            let last_player_position = if Vector2::ZERO != self.collision {
                zombie_position + self.collision
            } else {
                self.last_player_position
            };
            let bump_dir = zombie_position
                .direction_to(last_player_position)
                .normalized();
            self.bump();
            self.base_mut().look_at(player_position);
            bump_dir * self.current_speed * 1.5
        };
        if !self.moveable {
            return;
        }
        let speed = self.current_speed * 8.0;
        self.collision = Vector2::ZERO;
        if let Some(collision) = self.base.to_gd().move_and_collide(velocity) {
            // 发出排斥力的方向
            let from = collision.get_normal();
            if let Some(object) = collision.get_collider() {
                if is_boss(&object) {
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
                } else if not_boss(&object) {
                    let dir = (from + from.orthogonal()).normalized();
                    Self::zombie_collide(collision, dir * speed, 10);
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
        self.born_audio.play();
        self.guard();
        let mut animated_sprite2d = self.animated_sprite2d.bind_mut();
        animated_sprite2d.set_hurt_frames(self.hurt_frames.clone());
        animated_sprite2d.set_damage(BOSS_DAMAGE);
        if !self.boss_name.is_empty() {
            let name = self.boss_name.clone();
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
impl RustBoss {
    pub fn zombie_collide(collision: Gd<KinematicCollision2D>, velocity: Vector2, push_count: i32) {
        if 0 == push_count {
            return;
        }
        if let Some(object) = collision.get_collider() {
            if not_boss(&object) {
                let mut to_zombie = object.cast::<PhysicsBody2D>();
                let position = to_zombie.get_global_position();
                to_zombie.set_global_position(position + velocity);
                to_zombie.look_at(position + velocity);
                if let Some(another_collision) = to_zombie.move_and_collide(velocity) {
                    Self::zombie_collide(another_collision, velocity, push_count - 1);
                }
            }
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
        let speed = self.current_speed;
        //面对BOSS击退力下降50%
        let moved = direction * repel * 0.5;
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
        self.base_mut()
            .set_global_position(player_position + random_position(1000.0, 1100.0));
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

    pub fn update_hp_progress_hud(&mut self) {
        if !self.hud.is_instance_valid() {
            return;
        }
        self.hud
            .get_node_as::<Control>("HpProgress")
            .get_node_as::<ProgressBar>("ProgressBar")
            .set_value_no_signal((self.health as f64 / BOSS_MAX_HEALTH as f64) * 100.0);
    }

    #[func]
    pub fn get_current_direction(&self) -> Vector2 {
        let rotation = self.base().get_rotation();
        Vector2::new(rotation.cos(), rotation.sin())
    }

    fn notify_animation(&mut self) {
        self.animated_sprite2d
            .signals()
            .change_zombie_state()
            .emit(self.state);
        self.bump_damage_area
            .signals()
            .change_zombie_state()
            .emit(self.state);
    }
}
