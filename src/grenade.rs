use crate::player::RustPlayer;
use crate::{EXPLODE_AUDIOS, NO_NOISE, is_survivor, is_zombie};
use crossbeam_utils::atomic::AtomicCell;
use godot::builtin::{Callable, Vector2, real};
use godot::classes::node::PhysicsInterpolationMode;
use godot::classes::{
    AnimatedSprite2D, Area2D, AudioStreamPlayer2D, IRigidBody2D, Node2D, Object, RigidBody2D,
    TextureRect,
};
use godot::global::godot_error;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};

static NOISE_POSITION: AtomicCell<Vector2> = AtomicCell::new(NO_NOISE);

static ZOMBIE_NOISE_POSITION: AtomicCell<Vector2> = AtomicCell::new(NO_NOISE);

#[derive(GodotClass)]
#[class(base=RigidBody2D)]
pub struct RustGrenade {
    #[export]
    speed: real,
    #[export]
    countdown: f64,
    #[export]
    timed: bool,
    #[export]
    from_zombie: bool,
    bullet_point: Vector2,
    final_distance: real,
    final_repel: real,
    final_damage: i64,
    hit_area: OnReady<Gd<Area2D>>,
    damage_area: OnReady<Gd<Area2D>>,
    explode_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    explode_flash: OnReady<Gd<AnimatedSprite2D>>,
    texture_rect: OnReady<Gd<TextureRect>>,
    base: Base<RigidBody2D>,
}

#[godot_api]
impl IRigidBody2D for RustGrenade {
    fn init(base: Base<RigidBody2D>) -> Self {
        Self {
            speed: 50.0,
            countdown: 2.0,
            timed: true,
            from_zombie: false,
            bullet_point: Vector2::ZERO,
            final_distance: 0.0,
            final_repel: 0.0,
            final_damage: 0,
            hit_area: OnReady::from_node("HitArea"),
            damage_area: OnReady::from_node("DamageArea"),
            explode_audio: OnReady::from_node("ExplodeAudio"),
            explode_flash: OnReady::from_node("AnimatedSprite2D"),
            texture_rect: OnReady::from_node("TextureRect"),
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        if self.base().is_freeze_enabled() {
            if !self.explode_audio.is_playing() || !self.explode_flash.is_playing() {
                self.base_mut().queue_free();
            }
            return;
        }
        let bullet_point = self.bullet_point;
        let distance = self.final_distance;
        let current = self.base().get_global_position();
        if current.distance_to(bullet_point) >= distance {
            //到达最大距离
            self.explode();
        }
    }

    fn exit_tree(&mut self) {
        self.explode_audio.set_stream(Gd::null_arg());
        self.explode_audio.queue_free();
        self.explode_flash.queue_free();
    }

    fn ready(&mut self) {
        self.explode_flash.set_visible(false);
        self.base_mut()
            .set_physics_interpolation_mode(PhysicsInterpolationMode::ON);
        let mouse_position = self.get_mouse_position();
        self.base_mut().look_at(mouse_position);
        if self.timed {
            if let Some(mut tree) = self.base().get_tree() {
                if let Some(mut timer) = tree.create_timer(self.countdown) {
                    timer.connect("timeout", &self.base().callable("explode"));
                }
            }
        }
        let gd = self.to_gd();
        self.hit_area
            .signals()
            .body_entered()
            .connect_obj(&gd, Self::explode_ext);
        self.base_mut()
            .signals()
            .sleeping_state_changed()
            .connect_obj(&gd, Self::explode);
    }
}

#[godot_api]
impl RustGrenade {
    #[signal]
    pub fn sig();

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

    pub fn throw(&mut self, direction: Vector2) {
        let speed = self.speed;
        self.base_mut().apply_impulse(direction * speed);
    }

    #[func]
    pub fn explode_ext(&mut self, body: Gd<Node2D>) {
        if self.from_zombie {
            if is_survivor(&***body) {
                self.explode();
            }
            return;
        }
        if is_zombie(&***body) {
            self.explode();
        }
    }

    #[func]
    pub fn explode(&mut self) {
        if self.base().is_freeze_enabled() {
            return;
        }
        self.base_mut()
            .call_deferred("set_freeze_enabled", &[true.to_variant()]);
        self.base_mut().set_linear_velocity(Vector2::ZERO);
        //播放音效
        if self.explode_audio.get_stream().is_none() {
            #[allow(clippy::borrow_interior_mutable_const)]
            if let Some(audio) = EXPLODE_AUDIOS.pick_random() {
                self.explode_audio.set_stream(&audio);
            }
        }
        self.explode_audio.play();
        self.explode_flash.set_visible(true);
        self.explode_flash.set_global_rotation_degrees(0.0);
        self.explode_flash.play_ex().name("default").done();
        self.hit_area.queue_free();
        self.texture_rect.queue_free();
        let position = self.base().get_global_position();
        for mut body in self.damage_area.get_overlapping_bodies().iter_shared() {
            if !body.is_instance_valid() {
                continue;
            }
            if is_survivor(&***body) {
                body.cast::<RustPlayer>()
                    .bind_mut()
                    .on_hit(self.final_damage, position);
            } else if !self.from_zombie && is_zombie(&***body) {
                let direction = position.direction_to(body.get_global_position());
                body.call_deferred(
                    "on_hit",
                    &[
                        self.final_damage.to_variant(),
                        direction.to_variant(),
                        self.final_repel.to_variant(),
                        position.to_variant(),
                    ],
                );
                if self.final_damage > 0 {
                    RustPlayer::get().call_deferred("add_score", &[self.final_damage.to_variant()]);
                }
            } else if body.is_class("RustGrenade") || is_zombie(&***body) {
                // ok
            } else {
                godot_error!(
                    "{} hit an unexpected body: {}",
                    self.base().get_name(),
                    body.get_class()
                );
            }
        }
        if self.from_zombie {
            ZOMBIE_NOISE_POSITION.store(position);
            if let Some(mut tree) = self.base().get_tree() {
                if let Some(mut timer) = tree.create_timer(2.0) {
                    timer.connect(
                        "timeout",
                        &Callable::from_sync_fn("clean_zombie_grenade_noise", |_| {
                            ZOMBIE_NOISE_POSITION.store(NO_NOISE);
                            Ok(().to_variant())
                        }),
                    );
                }
            }
        } else {
            NOISE_POSITION.store(position);
            if let Some(mut tree) = self.base().get_tree() {
                if let Some(mut timer) = tree.create_timer(8.0) {
                    timer.connect(
                        "timeout",
                        &Callable::from_sync_fn("clean_grenade_noise", |_| {
                            NOISE_POSITION.store(NO_NOISE);
                            Ok(().to_variant())
                        }),
                    );
                }
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

    pub fn get_noise_position() -> Option<Vector2> {
        let r = NOISE_POSITION.load();
        if NO_NOISE == r { None } else { Some(r) }
    }

    pub fn get_zombie_noise_position() -> Option<Vector2> {
        let r = ZOMBIE_NOISE_POSITION.load();
        if NO_NOISE == r { None } else { Some(r) }
    }
}
