use crate::boss::RustBoss;
use crate::zombie::RustZombie;
use crate::zombie::animation::ZombieAnimation;
use godot::classes::{Area2D, IArea2D, Node, Node2D, Object};
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
            self.attack();
        }
    }

    #[func]
    pub fn on_area_2d_body_exited(&mut self, body: Gd<Node2D>) {
        if body.is_class("RustPlayer") {
            self.attack();
            if let Some(mut tree) = self.base().get_tree() {
                if let Some(mut timer) = tree.create_timer(0.5) {
                    timer.connect("timeout", &self.get_parent().callable("guard"));
                }
            }
        }
    }

    fn attack(&mut self) {
        // 攻击玩家
        if let Ok(mut zombie) = self.get_parent().try_cast::<RustZombie>() {
            if zombie.bind().is_face_to_user() {
                // 僵尸面向玩家才发起攻击
                zombie.bind_mut().attack();
            }
        } else if let Ok(mut boss) = self.get_parent().try_cast::<RustBoss>() {
            boss.bind_mut().attack();
        }
    }

    fn get_parent(&self) -> Gd<Node> {
        self.base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
    }

    fn get_zombie_animation(&self) -> Gd<ZombieAnimation> {
        self.base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
            .get_node_as::<ZombieAnimation>("AnimatedSprite2D")
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

    fn get_zombie_animation(&self) -> Gd<ZombieAnimation> {
        self.base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
            .get_node_as::<ZombieAnimation>("AnimatedSprite2D")
    }
}
