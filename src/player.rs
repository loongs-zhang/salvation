use crate::weapon::RustWeapon;
use crate::world::RustWorld;
use crate::{PLAYER_MAX_HEALTH, PlayerState};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Vector2, real};
use godot::classes::{
    AnimatedSprite2D, CharacterBody2D, ICharacterBody2D, Input, InputEvent, Node2D, Object,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};

static POSITION: AtomicCell<Vector2> = AtomicCell::new(Vector2::ZERO);

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPlayer {
    health: u32,
    state: PlayerState,
    speed: real,
    animated_sprite2d: OnReady<Gd<AnimatedSprite2D>>,
    weapon: OnReady<Gd<Node2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            health: PLAYER_MAX_HEALTH,
            state: PlayerState::Born,
            speed: 200.0,
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            weapon: OnReady::from_node("Weapon"),
            base,
        }
    }

    fn ready(&mut self) {
        if let Some(tree) = self.base().get_parent() {
            tree.cast::<RustWorld>()
                .signals()
                .change_player_state()
                .connect_self(RustWorld::on_change_player_state);
        }
    }

    fn process(&mut self, _delta: f64) {
        if PlayerState::Dead == self.state {
            return;
        }
        let player_position = self.base().get_global_position();
        POSITION.store(player_position);
        let mouse_position = self.get_mouse_position();
        self.weapon.look_at(mouse_position);
        let input = Input::singleton();
        if input.is_action_pressed("mouse_left") {
            self.shoot();
        }
        let dir = Vector2::new(
            input.get_axis("move_left", "move_right"),
            input.get_axis("move_up", "move_down"),
        );
        match self.state {
            PlayerState::Born | PlayerState::Guard | PlayerState::Shoot | PlayerState::Dead => {
                self.animated_sprite2d.look_at(mouse_position)
            }
            PlayerState::Run => self.animated_sprite2d.look_at(player_position + dir),
        }
        let mut character_body2d = self.base.to_gd();
        if dir != Vector2::ZERO {
            character_body2d.set_velocity(dir.normalized() * self.speed);
        } else {
            character_body2d.set_velocity(Vector2::ZERO);
            self.guard();
        }
        character_body2d.move_and_slide();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("shift") || event.is_action_pressed("mouse_right") {
            self.run();
        } else if event.is_action_released("shift")
            || event.is_action_released("mouse_left")
            || event.is_action_released("mouse_right")
        {
            self.guard();
        }
    }
}

#[godot_api]
impl RustPlayer {
    #[signal]
    pub fn hit(hit_val: i64);

    #[func]
    pub fn on_hit(&mut self, hit_val: i64) {
        let health = self.health;
        self.health = if hit_val > 0 {
            health.saturating_sub(hit_val as u32)
        } else {
            health.saturating_add(-hit_val as u32)
        };
        if 0 == self.health {
            self.die();
        }
    }

    #[func]
    pub fn born(&mut self) {
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 200.0;
        self.state = PlayerState::Born;
        self.health = PLAYER_MAX_HEALTH;
        self.notify_zombies();
    }

    pub fn guard(&mut self) {
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 200.0;
        self.state = PlayerState::Guard;
    }

    pub fn run(&mut self) {
        self.animated_sprite2d.play_ex().name("run").done();
        self.speed = 300.0;
        self.state = PlayerState::Run;
    }

    pub fn shoot(&mut self) {
        self.animated_sprite2d.play_ex().name("guard").done();
        self.speed = 100.0;
        self.state = PlayerState::Shoot;
        self.weapon
            .get_node_as::<RustWeapon>("RustWeapon")
            .bind_mut()
            .fire();
    }

    pub fn die(&mut self) {
        self.animated_sprite2d.play_ex().name("die").done();
        self.speed = 0.0;
        self.state = PlayerState::Dead;
        self.notify_zombies();
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(3.0) {
                timer.connect("timeout", &self.base().callable("born"));
            }
        }
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        self.base().get_canvas_transform().affine_inverse()
            * self
                .base()
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }

    fn notify_zombies(&mut self) {
        if let Some(tree) = self.base().get_parent() {
            tree.cast::<RustWorld>()
                .signals()
                .change_player_state()
                .emit(self.state);
        }
    }

    pub fn get_position() -> Vector2 {
        POSITION.load()
    }
}
