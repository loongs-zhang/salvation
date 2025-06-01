use crate::player::RustPlayer;
use crate::zombie::boss::RustBoss;
use crate::{is_boss, not_boss};
use godot::builtin::{Vector2, real};
use godot::classes::node::PhysicsInterpolationMode;
use godot::classes::{
    Area2D, AudioStreamPlayer2D, CollisionShape2D, IArea2D, INode2D, Node2D, Object, RayCast2D,
};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustBullet {
    #[export]
    speed: real,
    bullet_point: Vector2,
    final_distance: real,
    final_repel: real,
    final_damage: i64,
    final_penetrate: real,
    hit_count: i16,
    direction: Vector2,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustBullet {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            speed: 1000.0,
            bullet_point: Vector2::ZERO,
            final_distance: 0.0,
            final_repel: 0.0,
            final_damage: 0,
            final_penetrate: 0.0,
            hit_count: 0,
            direction: Vector2::ZERO,
            base,
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let direction = self.direction;
        let speed = self.speed;
        let bullet_point = self.bullet_point;
        let distance = self.final_distance;
        let mut base_mut = self.base_mut();
        let current = base_mut.get_global_position();
        let new_position = current
            + Vector2::new(
                direction.x * delta as f32 * speed,
                direction.y * delta as f32 * speed,
            );
        if new_position.distance_to(bullet_point) >= distance {
            //到达最大距离
            base_mut.queue_free();
            return;
        }
        base_mut.set_global_position(new_position);
    }

    fn ready(&mut self) {
        self.base_mut()
            .set_physics_interpolation_mode(PhysicsInterpolationMode::ON);
        let mouse_position = self.get_mouse_position();
        self.base_mut().look_at(mouse_position);
    }
}

#[godot_api]
impl RustBullet {
    pub fn set_bullet_point(&mut self, bullet_point: Vector2) {
        self.bullet_point = bullet_point;
    }

    pub fn set_final_distance(&mut self, distance: real) {
        self.final_distance = distance;
    }

    pub fn set_final_damage(&mut self, damage: i64) {
        self.final_damage = damage;
    }

    pub fn set_final_repel(&mut self, final_repel: real) {
        self.final_repel = final_repel;
    }

    pub fn set_final_penetrate(&mut self, penetrate: real) {
        self.final_penetrate = penetrate;
    }

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

    fn on_hit(&mut self, cost: i16) {
        self.hit_count += cost;
        if self.hit_count >= self.final_penetrate as i16 {
            //达到最大穿透上限
            self.base_mut().queue_free()
        }
    }
}

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct BulletDamageArea {
    collision_shape2d: OnReady<Gd<CollisionShape2D>>,
    headshot_ray1: OnReady<Gd<RayCast2D>>,
    headshot_ray2: OnReady<Gd<RayCast2D>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for BulletDamageArea {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            collision_shape2d: OnReady::from_node("CollisionShape2D"),
            headshot_ray1: OnReady::from_node("HeadshotRay1"),
            headshot_ray2: OnReady::from_node("HeadshotRay2"),
            hit_audio: OnReady::from_node("HitAudio"),
            base,
        }
    }

    fn exit_tree(&mut self) {
        self.collision_shape2d.queue_free();
        self.headshot_ray1.queue_free();
        self.headshot_ray2.queue_free();
        self.hit_audio.set_stream(Gd::null_arg());
        self.hit_audio.queue_free();
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
    pub fn on_area_2d_body_entered(&mut self, mut body: Gd<Node2D>) {
        let mut rust_bullet = self
            .base()
            .get_parent()
            .expect("RustBullet not found")
            .cast::<RustBullet>();
        let mut damage = 0;
        if not_boss(&***body) {
            damage = rust_bullet.bind().final_damage;
            self.hit_audio.play();
            rust_bullet.bind_mut().on_hit(1);
            if self.headshot_ray1.is_colliding() || self.headshot_ray2.is_colliding() {
                RustPlayer::get().bind_mut().headshot();
                damage *= 3;
            }
            if !body.is_instance_valid() {
                return;
            }
            body.call_deferred(
                "on_hit",
                &[
                    damage.to_variant(),
                    rust_bullet.bind().direction.to_variant(),
                    rust_bullet.bind().final_repel.to_variant(),
                    rust_bullet.get_global_position().to_variant(),
                ],
            );
        } else if is_boss(&***body) {
            damage = rust_bullet.bind().final_damage;
            self.hit_audio.play();
            // BOSS身体大，消耗更多穿透
            rust_bullet.bind_mut().on_hit(2);
            if self.headshot_ray1.is_colliding() || self.headshot_ray2.is_colliding() {
                RustPlayer::get().bind_mut().headshot();
                damage *= 3;
            }
            body.cast::<RustBoss>().bind_mut().on_hit(
                damage,
                rust_bullet.bind().direction,
                rust_bullet.bind().final_repel,
                rust_bullet.get_global_position(),
            );
        }
        if damage > 0 {
            RustPlayer::get().call_deferred("add_score", &[damage.to_variant()]);
        }
    }
}
