use godot::builtin::{Vector2, real};
use godot::classes::{
    AnimatedSprite2D, Camera2D, CharacterBody2D, CollisionShape2D, ICharacterBody2D, Input,
    InputEvent,
};
use godot::obj::{Base, Gd, OnReady};
use godot::register::{GodotClass, godot_api};

#[derive(Default)]
enum PlayerState {
    #[default]
    Guard,
    Run,
    Shoot,
}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct RustPlayer {
    state: PlayerState,
    speed: real,
    animated_sprite2d: OnReady<Gd<AnimatedSprite2D>>,
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    camera2d: OnReady<Gd<Camera2D>>,
    base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for RustPlayer {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            state: PlayerState::Guard,
            speed: 200.0,
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            camera2d: OnReady::from_node("Camera2D"),
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        // Note: exact=false by default, in Rust we have to provide it explicitly
        let input = Input::singleton();
        self.animated_sprite2d.look_at(
            self.camera2d.get_canvas_transform().affine_inverse()
                * self
                    .camera2d
                    .get_viewport()
                    .expect("Viewport not found")
                    .get_mouse_position(),
        );
        let dir = Vector2::new(
            input.get_axis("move_left", "move_right"),
            input.get_axis("move_up", "move_down"),
        );
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
        if event.is_action_pressed("mouse_left") {
            self.shoot();
        } else if event.is_action_pressed("shift") || event.is_action_pressed("mouse_right") {
            match self.state {
                PlayerState::Guard | PlayerState::Shoot => self.run(),
                PlayerState::Run => self.guard(),
            }
        }
    }
}

#[godot_api]
impl RustPlayer {
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
    }
}
