use crate::bullet::RustBullet;
use godot::builtin::{Vector2, real};
use godot::classes::{
    AnimatedSprite2D, Camera2D, CharacterBody2D, CollisionShape2D, ICharacterBody2D, Input,
    InputEvent, Node2D, PackedScene,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
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
    bullet_scene: OnReady<Gd<PackedScene>>,
    bullet_point: OnReady<Gd<Node2D>>,
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
            bullet_scene: OnReady::from_loaded("res://scenes/rust_bullet.tscn"),
            bullet_point: OnReady::from_node("BulletPoint"),
            animated_sprite2d: OnReady::from_node("AnimatedSprite2D"),
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            camera2d: OnReady::from_node("Camera2D"),
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        let mouse_vector2 = self.get_mouse_vector2();
        // Note: exact=false by default, in Rust we have to provide it explicitly
        let input = Input::singleton();
        self.animated_sprite2d.look_at(mouse_vector2);
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

        if let Some(mut bullet) = self.bullet_scene.try_instantiate_as::<RustBullet>() {
            // fixme 修复角色转向时子弹的发出点
            bullet.set_global_position(self.bullet_point.get_global_position());
            let vector2 = self
                .base()
                .get_global_position()
                .direction_to(self.get_mouse_vector2());
            bullet.bind_mut().set_vector2(vector2);
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&bullet);
                }
            }
        }
    }

    pub fn get_mouse_vector2(&self) -> Vector2 {
        self.camera2d.get_canvas_transform().affine_inverse()
            * self
                .camera2d
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }
}
