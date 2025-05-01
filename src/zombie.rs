use crate::player::RustPlayer;
use dashmap::DashMap;
use godot::builtin::{GString, Vector2, real};
use godot::classes::{
    AnimatedSprite2D, Area2D, CharacterBody2D, CollisionShape2D, IAnimatedSprite2D, IArea2D,
    ICharacterBody2D, Node2D, Object,
};
use godot::global::godot_print;
use godot::obj::{Base, Gd, InstanceId, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, GodotConvert, godot_api};
use rand::Rng;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const DAMAGE: i64 = 10;

const MAX_HEALTH: u32 = 100;

static ZOMBIE_HEALTH: LazyLock<DashMap<InstanceId, u32>> = LazyLock::new(DashMap::default);

#[derive(GodotConvert, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
#[godot(via = GString)]
pub enum ZombieState {
    #[default]
    Guard,
    Run,
    Attack,
    Dead,
}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustZombie {
    last_turn_time: Instant,
    turn_cooldown: Duration,
    state: ZombieState,
    speed: real,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    area2d: OnReady<Gd<ZombieAttackArea>>,
    animated_sprite2d: OnReady<Gd<ZombieAnimation>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustZombie {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            last_turn_time: Instant::now(),
            turn_cooldown: Duration::from_secs(5),
            state: ZombieState::Guard,
            speed: 50.0,
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            area2d: OnReady::from_node("Area2D"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            base,
        }
    }

    fn ready(&mut self) {
        self.last_turn_time -= self.turn_cooldown;
        self.animated_sprite2d
            .signals()
            .state_change()
            .connect_self(ZombieAnimation::on_state_change);
        _ = ZOMBIE_HEALTH.insert(self.animated_sprite2d.instance_id(), MAX_HEALTH);
    }

    fn physics_process(&mut self, _delta: f64) {
        if ZombieState::Attack == self.state || ZombieState::Dead == self.state {
            return;
        }
        if let Some(health) = ZOMBIE_HEALTH.get(&self.animated_sprite2d.instance_id()) {
            if 0 == *health {
                self.die();
                return;
            }
        }
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        let distance = zombie_position.distance_to(player_position);
        let rotation = self.base().get_rotation();
        let current_zombie_dir = Vector2::new(rotation.cos(), rotation.sin());
        let to_player_dir = zombie_position.direction_to(player_position).normalized();
        let angle = current_zombie_dir.angle_to(to_player_dir).to_degrees();
        let mut character_body2d = self.base.to_gd();
        if -60.0 < angle && angle < 60.0 && distance <= 200.0 {
            // 跑向玩家
            self.run();
            character_body2d.set_velocity(to_player_dir * self.speed);
        } else {
            // 无目的移动
            self.guard();
            let now = Instant::now();
            if now.duration_since(self.last_turn_time) >= self.turn_cooldown {
                let dir = Vector2::new(
                    rand::thread_rng().gen_range(-1.0..1.0),
                    rand::thread_rng().gen_range(-1.0..1.0),
                );
                self.base_mut().look_at(zombie_position + dir);
                character_body2d.set_velocity(dir.normalized() * self.speed);
                self.last_turn_time = now;
            }
        }
        character_body2d.move_and_slide();
    }
}

#[godot_api]
impl RustZombie {
    pub fn guard(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 50.0;
        self.state = ZombieState::Guard;
        self.animated_sprite2d
            .signals()
            .state_change()
            .emit(self.state);
    }

    pub fn run(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("run").done();
        self.speed = 250.0;
        self.state = ZombieState::Run;
        self.animated_sprite2d
            .signals()
            .state_change()
            .emit(self.state);
    }

    pub fn attack(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("attack").done();
        self.speed = 50.0;
        self.state = ZombieState::Attack;
        self.animated_sprite2d
            .signals()
            .state_change()
            .emit(self.state);
    }

    pub fn die(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("die").done();
        self.speed = 0.0;
        self.state = ZombieState::Dead;
        self.collision_shape2d.queue_free();
        self.animated_sprite2d
            .signals()
            .state_change()
            .emit(self.state);
    }

    pub fn get_distance(&self) -> real {
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        zombie_position.distance_to(player_position)
    }

    pub fn is_face_to_face(angle: real) -> bool {
        (-60.0..=60.0).contains(&angle)
    }
}

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct ZombieAttackArea {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for ZombieAttackArea {
    fn init(base: Base<Area2D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        self.signals()
            .body_entered()
            .connect_self(Self::on_area_2d_body_entered);
        self.signals()
            .body_exited()
            .connect_self(Self::on_area_2d_body_exited);
    }
}

#[godot_api]
impl ZombieAttackArea {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        godot_print!("ZombieAttackArea body entered: {:?}", body);
        if body.is_class("RustPlayer") {
            // 攻击玩家
            self.get_zombie().bind_mut().attack();
        }
    }

    #[func]
    pub fn on_area_2d_body_exited(&mut self, body: Gd<Node2D>) {
        if body.is_class("RustPlayer") {
            self.get_zombie().bind_mut().attack();
            if let Some(mut tree) = self.base().get_tree() {
                if let Some(mut timer) = tree.create_timer(0.5) {
                    timer.connect("timeout", &self.base().callable("back_to_guard"));
                }
            }
        }
    }

    #[func]
    pub fn back_to_guard(&mut self) {
        self.get_zombie().bind_mut().guard();
    }

    fn get_zombie(&mut self) -> Gd<RustZombie> {
        self.base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
            .cast::<RustZombie>()
    }
}

const HURT_FRAME: [i32; 4] = [2, 3, 4, 5];

#[derive(GodotClass)]
#[class(base=AnimatedSprite2D)]
pub struct ZombieAnimation {
    state: ZombieState,
    base: Base<AnimatedSprite2D>,
}

#[godot_api]
impl IAnimatedSprite2D for ZombieAnimation {
    fn init(base: Base<AnimatedSprite2D>) -> Self {
        Self {
            state: ZombieState::Guard,
            base,
        }
    }

    fn ready(&mut self) {
        self.signals()
            .frame_changed()
            .connect_self(Self::on_animated_sprite_2d_frame_changed);
        if let Some(tree) = self.base().get_parent() {
            if let Some(tree) = tree.get_parent() {
                tree.get_node_as::<RustPlayer>("RustPlayer")
                    .signals()
                    .hit()
                    .connect_self(RustPlayer::on_hit);
            }
        }
    }
}

#[godot_api]
impl ZombieAnimation {
    #[signal]
    pub fn state_change(state: ZombieState);

    #[func]
    pub fn on_state_change(&mut self, state: ZombieState) {
        self.state = state;
    }

    #[func]
    pub fn on_animated_sprite_2d_frame_changed(&mut self) {
        let base = self.base();
        let distance = self.get_distance();
        if ZombieState::Attack == self.state
            && distance <= 120.0
            && base.get_animation() == "attack".into()
            && HURT_FRAME.contains(&base.get_frame())
        {
            if RustPlayer::is_dead() {
                //todo 通过信号重新开始播放动画
                if let Some(mut frames) = base.get_sprite_frames() {
                    frames.set_animation_loop("attack", false);
                }
                return;
            }
            if let Some(tree) = base.get_parent() {
                if let Some(tree) = tree.get_parent() {
                    // 伤害玩家
                    godot_print!(
                        "zombie attack player, damage:{} distance:{} frame:{}",
                        DAMAGE,
                        distance,
                        base.get_frame()
                    );
                    tree.get_node_as::<RustPlayer>("RustPlayer")
                        .signals()
                        .hit()
                        .emit(DAMAGE);
                }
            }
        }
    }

    pub fn get_distance(&self) -> real {
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        zombie_position.distance_to(player_position)
    }
}
