use crate::player::RustPlayer;
use godot::builtin::{Vector2, real};
use godot::classes::{
    AnimatedSprite2D, Area2D, CharacterBody2D, IArea2D, ICharacterBody2D, Node2D, Object,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use rand::Rng;

#[derive(Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum ZombieState {
    #[default]
    Guard,
    Run,
    Attack,
    Dead,
}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustZombie {
    state: ZombieState,
    speed: real,
    animated_sprite2d: OnReady<Gd<AnimatedSprite2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustZombie {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            state: ZombieState::Guard,
            speed: 50.0,
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            base,
        }
    }

    fn physics_process(&mut self, _delta: f64) {
        if self.state == ZombieState::Attack || self.state == ZombieState::Dead {
            return;
        }
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        let distance = zombie_position.distance_to(player_position);
        let mut character_body2d = self.base.to_gd();
        if distance <= 200.0 {
            // 跑向玩家
            self.base_mut().look_at(player_position);
            self.run();
            character_body2d.set_velocity(
                zombie_position.direction_to(player_position).normalized() * self.speed,
            );
        } else {
            // 无目的移动
            self.guard();
            if rand::thread_rng().gen_range(0..100) < 1 {
                let dir = Vector2::new(
                    rand::thread_rng().gen_range(-1.0..1.0),
                    rand::thread_rng().gen_range(-1.0..1.0),
                );
                self.base_mut().look_at(zombie_position + dir);
                character_body2d.set_velocity(dir.normalized() * self.speed);
            }
        }
        character_body2d.move_and_slide();
    }
}

#[godot_api]
impl RustZombie {
    pub fn guard(&mut self) {
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 50.0;
        self.state = ZombieState::Guard;
    }

    pub fn run(&mut self) {
        self.animated_sprite2d.play_ex().name("run").done();
        self.speed = 250.0;
        self.state = ZombieState::Run;
    }

    pub fn attack(&mut self) {
        self.animated_sprite2d.play_ex().name("attack").done();
        self.speed = 50.0;
        self.state = ZombieState::Attack;
    }

    pub fn die(&mut self) {
        self.animated_sprite2d.play_ex().name("die").done();
        self.speed = 0.0;
        self.state = ZombieState::Dead;
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
    pub fn hit();

    #[func]
    fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        if body.is_class("RustPlayer") {
            // 攻击玩家
            let mut zombie = self.get_zombie();
            zombie.look_at(RustPlayer::get_position());
            zombie.bind_mut().attack();
        }
    }

    #[func]
    fn on_area_2d_body_exited(&mut self, body: Gd<Node2D>) {
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
    fn back_to_guard(&mut self) {
        self.get_zombie().bind_mut().guard();
    }

    fn get_zombie(&mut self) -> Gd<RustZombie> {
        self.base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
            .cast::<RustZombie>()
    }
}
