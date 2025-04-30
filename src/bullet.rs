use godot::builtin::{Vector2, real};
use godot::classes::{INode2D, Node2D};
use godot::obj::{Base, WithBaseField};
use godot::prelude::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustBullet {
    speed: real,
    direction: Vector2,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustBullet {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            speed: 400.0,
            direction: Vector2::ONE,
            base,
        }
    }

    fn ready(&mut self) {
        let mouse_position = self.get_mouse_position();
        self.base_mut().look_at(mouse_position);
    }

    fn physics_process(&mut self, delta: f64) {
        let vector2 = self.direction;
        let speed = self.speed;
        let mut base_mut = self.base_mut();
        let current = base_mut.get_global_position();
        base_mut.set_global_position(
            current
                + Vector2::new(
                    vector2.x * delta as f32 * speed,
                    vector2.y * delta as f32 * speed,
                ),
        );
    }
}

#[godot_api]
impl RustBullet {
    #[func]
    pub fn set_direction(&mut self, direction: Vector2) {
        self.direction = direction;
    }

    pub fn get_mouse_position(&self) -> Vector2 {
        self.base().get_canvas_transform().affine_inverse()
            * self
                .base()
                .get_viewport()
                .expect("Viewport not found")
                .get_mouse_position()
    }
}
