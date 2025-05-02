use crate::BULLET_DAMAGE;
use crate::zombie::RustZombie;
use godot::builtin::{Vector2, real};
use godot::classes::{Area2D, IArea2D, INode2D, Node2D, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
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

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct BulletDamageArea {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for BulletDamageArea {
    fn init(base: Base<Area2D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        self.signals()
            .body_entered()
            .connect_self(Self::on_area_2d_body_entered);
    }
}

#[godot_api]
impl BulletDamageArea {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        if body.is_class("RustZombie") {
            body.cast::<RustZombie>().bind_mut().on_hit(BULLET_DAMAGE);
            let mut rust_bullet = self.base().get_parent().expect("RustBullet not found");
            rust_bullet.queue_free();
        }
    }
}
