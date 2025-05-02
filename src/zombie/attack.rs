use crate::zombie::RustZombie;
use crate::zombie::animation::ZombieAnimation;
use godot::classes::{Area2D, IArea2D, Node2D, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

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
        self.get_zombie_animation()
            .signals()
            .player_in_area()
            .connect_self(ZombieAnimation::on_player_in_area);
    }
}

#[godot_api]
impl ZombieAttackArea {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        if body.is_class("RustPlayer") {
            self.get_zombie_animation()
                .signals()
                .player_in_area()
                .emit(true);
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

    fn get_zombie(&self) -> Gd<RustZombie> {
        self.base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
            .cast::<RustZombie>()
    }

    fn get_zombie_animation(&mut self) -> Gd<ZombieAnimation> {
        self.get_zombie().bind().get_animated_sprite2d()
    }
}

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct ZombieDamageArea {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for ZombieDamageArea {
    fn init(base: Base<Area2D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        self.signals()
            .body_exited()
            .connect_self(Self::on_area_2d_body_exited);
        self.get_zombie_animation()
            .signals()
            .player_in_area()
            .connect_self(ZombieAnimation::on_player_in_area);
    }
}

#[godot_api]
impl ZombieDamageArea {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_area_2d_body_exited(&mut self, body: Gd<Node2D>) {
        if body.is_class("RustPlayer") {
            self.get_zombie_animation()
                .signals()
                .player_in_area()
                .emit(false);
        }
    }

    fn get_zombie(&self) -> Gd<RustZombie> {
        self.base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
            .cast::<RustZombie>()
    }

    fn get_zombie_animation(&mut self) -> Gd<ZombieAnimation> {
        self.get_zombie().bind().get_animated_sprite2d()
    }
}
