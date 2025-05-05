use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::animation::ZombieAnimation;
use crate::zombie::attack::{ZombieAttackArea, ZombieDamageArea};
use crate::{PlayerState, ZOMBIE_MAX_HEALTH, ZOMBIE_RAMPAGE_TIME, ZombieState};
use godot::builtin::{Vector2, real};
use godot::classes::{CharacterBody2D, CollisionShape2D, ICharacterBody2D};
use godot::global::godot_print;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use std::time::{Duration, Instant};

pub mod attack;

pub mod animation;

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustZombie {
    #[export]
    health: u32,
    #[export]
    rampage_time: u32,
    create_time: Instant,
    last_turn_time: Instant,
    turn_cooldown: Duration,
    state: ZombieState,
    speed: real,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    zombie_attack_area: OnReady<Gd<ZombieAttackArea>>,
    zombie_damage_area: OnReady<Gd<ZombieDamageArea>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustZombie {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            health: ZOMBIE_MAX_HEALTH,
            rampage_time: ZOMBIE_RAMPAGE_TIME,
            create_time: Instant::now(),
            last_turn_time: Instant::now(),
            turn_cooldown: Duration::from_secs(5),
            state: ZombieState::Guard,
            speed: 20.0,
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            zombie_attack_area: OnReady::from_node("ZombieAttackArea"),
            zombie_damage_area: OnReady::from_node("ZombieDamageArea"),
            base,
        }
    }

    fn ready(&mut self) {
        self.last_turn_time -= self.turn_cooldown;
        self.animated_sprite2d
            .signals()
            .change_zombie_state()
            .connect_self(ZombieAnimation::on_change_zombie_state);
        godot_print!(
            "Zombie {:?} created at {:?}",
            self.base().get_global_position(),
            self.create_time
        );
    }

    fn physics_process(&mut self, _delta: f64) {
        if ZombieState::Dead == self.state || ZombieState::Attack == self.state {
            return;
        }
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        let distance = zombie_position.distance_to(player_position);
        let rotation = self.base().get_rotation();
        let current_zombie_dir = Vector2::new(rotation.cos(), rotation.sin());
        let to_player_dir = zombie_position.direction_to(player_position).normalized();
        let angle = current_zombie_dir.angle_to(to_player_dir).to_degrees();
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
                        if ZombieState::Run == to_zombie.bind().state {
                            continue;
                        }
                        // 给其他僵尸让开位置
                        let dir = from.orthogonal();
                        let speed = to_zombie.bind().get_speed();
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
        if PlayerState::Dead == RustPlayer::get_state() {
            //玩家死亡时，僵尸往玩家相反的方向移动一段距离
            self.guard();
            let from_player_dir = player_position.direction_to(zombie_position).normalized();
            character_body2d.look_at(zombie_position + from_player_dir);
            character_body2d.set_velocity(from_player_dir * self.speed);
        } else if distance <= 200.0 && Self::is_face_to_face(angle) || self.can_rampage() {
            // 跑向玩家
            self.run();
            character_body2d.set_velocity(to_player_dir * self.speed);
        } else {
            // 无目的移动
            self.guard();
            let now = Instant::now();
            if now.duration_since(self.last_turn_time) >= self.turn_cooldown {
                let direction = Self::random_direction();
                character_body2d.look_at(zombie_position + direction);
                character_body2d.set_velocity(direction * self.speed);
                self.last_turn_time = now;
            }
        }
        character_body2d.move_and_slide();
    }
}

#[godot_api]
impl RustZombie {
    #[func]
    pub fn on_hit(&mut self, hit_val: i64, direction: Vector2, repel: real) {
        let health = self.health;
        self.health = if hit_val > 0 {
            health.saturating_sub(hit_val as u32)
        } else {
            health.saturating_add(-hit_val as u32)
        };
        let zombie_position = self.base().get_global_position();
        let moved = direction * repel;
        let new_position = zombie_position + moved;
        let mut base_mut = self.base_mut();
        base_mut.look_at(zombie_position - direction);
        base_mut.set_global_position(new_position);
        drop(base_mut);
        if 0 == self.health {
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
        self.speed = 20.0;
        self.state = ZombieState::Guard;
        self.notify_animation();
    }

    pub fn run(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("run").done();
        self.speed = 250.0;
        self.state = ZombieState::Run;
        self.notify_animation();
    }

    pub fn attack(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("attack").done();
        self.speed = 50.0;
        self.state = ZombieState::Attack;
        self.notify_animation();
    }

    pub fn die(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("die").done();
        self.speed = 0.0;
        self.state = ZombieState::Dead;
        // 释放资源
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
    }

    pub fn can_rampage(&self) -> bool {
        if PlayerState::Dead == RustPlayer::get_state() {
            return false;
        }
        RustLevel::can_rampage()
            || Instant::now().duration_since(self.create_time).as_millis() as u32
                >= self.rampage_time
    }

    pub fn get_distance(&self) -> real {
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        zombie_position.distance_to(player_position)
    }

    pub fn get_speed(&self) -> real {
        self.speed
    }

    pub fn is_face_to_face(angle: real) -> bool {
        (-60.0..=60.0).contains(&angle)
    }
}
