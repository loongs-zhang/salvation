use crate::boss::bump::BossBumpArea;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::zombie::RustZombie;
use crate::zombie::animation::ZombieAnimation;
use crate::zombie::attack::{ZombieAttackArea, ZombieDamageArea};
use crate::zombie::hit::ZombieHit;
use crate::{
    BOSS_BUMP_DISTANCE, BOSS_DAMAGE, BOSS_MAX_BODY_COUNT, BOSS_MAX_HEALTH, BOSS_MOVE_SPEED,
    PlayerState, ZOMBIE_MAX_DISTANCE, ZombieState,
};
use godot::builtin::{Vector2, real};
use godot::classes::{
    AudioStreamPlayer2D, CharacterBody2D, CollisionShape2D, GpuParticles2D, ICharacterBody2D,
    PackedScene,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub mod bump;

static BODY_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustBoss {
    #[export]
    health: u32,
    #[export]
    speed: real,
    state: ZombieState,
    current_speed: real,
    hurt_frames: Vec<i32>,
    last_player_position: Vector2,
    last_record_time: Instant,
    record_cooldown: Duration,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    zombie_attack_area: OnReady<Gd<ZombieAttackArea>>,
    zombie_damage_area: OnReady<Gd<ZombieDamageArea>>,
    bump_damage_area: OnReady<Gd<BossBumpArea>>,
    hit_scene: OnReady<Gd<PackedScene>>,
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
            speed: BOSS_MOVE_SPEED,
            health: BOSS_MAX_HEALTH,
            state: ZombieState::Guard,
            current_speed: BOSS_MOVE_SPEED * 0.2,
            // hurt_frames: vec![26, 27, 28, 29, 30],
            hurt_frames: vec![2, 3, 4, 5],
            last_player_position: Vector2::ZERO,
            last_record_time: Instant::now(),
            record_cooldown: Duration::from_millis(1000),
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            zombie_attack_area: OnReady::from_node("ZombieAttackArea"),
            zombie_damage_area: OnReady::from_node("ZombieDamageArea"),
            bump_damage_area: OnReady::from_node("BossBumpArea"),
            hit_scene: OnReady::from_loaded("res://scenes/zombie_hit.tscn"),
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

    fn ready(&mut self) {
        let mut animated_sprite2d = self.animated_sprite2d.bind_mut();
        animated_sprite2d.set_hurt_frames(self.hurt_frames.clone());
        animated_sprite2d.set_damage(BOSS_DAMAGE);
        animated_sprite2d
            .signals()
            .change_zombie_state()
            .connect_self(ZombieAnimation::on_change_zombie_state);
        drop(animated_sprite2d);
        self.bump_damage_area
            .signals()
            .change_zombie_state()
            .connect_self(BossBumpArea::on_change_zombie_state);
        self.guard();
    }

    fn process(&mut self, _delta: f64) {
        if ZombieState::Dead == self.state || RustWorld::is_paused() {
            if BODY_COUNT.load(Ordering::Acquire) >= BOSS_MAX_BODY_COUNT {
                self.clean_body();
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
        let player_position = RustPlayer::get_position();
        let now = Instant::now();
        if now.duration_since(self.last_record_time) >= self.record_cooldown {
            self.last_player_position = player_position;
            self.last_record_time = now;
        }
        let zombie_position = self.base().get_global_position();
        let distance = zombie_position.distance_to(player_position);
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
                        let mut to_zombie = object.cast::<RustZombie>();
                        // 其他僵尸让开位置，这里分2步走是为了规避一些碰撞检测
                        let dir = from.orthogonal();
                        let speed = to_zombie.bind().get_current_speed();
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
        } else if distance >= BOSS_BUMP_DISTANCE {
            // 走向玩家
            self.guard();
            self.base_mut().look_at(player_position);
            character_body2d.set_velocity(to_player_dir * self.current_speed);
        } else {
            // 冲撞玩家
            let last_player_position = self.last_player_position;
            let bump_dir = zombie_position
                .direction_to(last_player_position)
                .normalized();
            self.bump();
            self.base_mut().look_at(player_position);
            character_body2d.set_velocity(bump_dir * self.current_speed * 1.5);
        }
        character_body2d.move_and_slide();
    }
}

#[godot_api]
impl RustBoss {
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
        //面对BOSS击退力下降50%
        let moved = direction * repel * 0.5;
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

    #[func]
    pub fn guard(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed;
        self.state = ZombieState::Guard;
        if !self.guard_audio.is_playing() && self.guard_audio.is_inside_tree() {
            self.guard_audio.play();
        }
        self.notify_animation();
    }

    pub fn bump(&mut self) {
        if ZombieState::Run == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("bump").done();
        self.current_speed = self.speed * 1.5;
        self.state = ZombieState::Run;
        if !self.bump_audio.is_playing() && self.bump_audio.is_inside_tree() {
            self.bump_audio.play();
        }
        self.notify_animation();
    }

    pub fn hit(&mut self, direction: Vector2, hit_position: Vector2) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.2;
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

    pub fn attack(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("attack").done();
        self.current_speed = self.speed * 0.75;
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
        self.bump_damage_area.queue_free();
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
            .kill_boss_confirmed();
        BODY_COUNT.fetch_add(1, Ordering::Release);
        // 45S后自动清理尸体
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(45.0) {
                timer.connect("timeout", &self.base().callable("clean_body"));
            }
        }
    }

    #[func]
    pub fn clean_body(&mut self) {
        self.base_mut().queue_free();
        BODY_COUNT.fetch_sub(1, Ordering::Release);
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

    pub fn flash(&mut self) {
        let player_position = RustPlayer::get_position();
        self.base_mut().set_global_position(
            player_position
                + Vector2::new(Self::random_half_position(), Self::random_half_position()),
        );
    }

    fn random_half_position() -> real {
        let mut rng = rand::thread_rng();
        if rng.gen_range(-1.0..1.0) >= 0.0 {
            rng.gen_range(1000.0..1100.0)
        } else {
            rng.gen_range(-1100.0..-1000.0)
        }
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
