use crate::zombie::boomer::RustBoomer;
use godot::classes::{Area2D, IArea2D, Node, Node2D, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct ZombieExplodeArea {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for ZombieExplodeArea {
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
impl ZombieExplodeArea {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        if body.is_class("RustPlayer") {
            if let Ok(mut boomer) = self.get_parent().try_cast::<RustBoomer>() {
                if !boomer.bind().is_face_to_user() {
                    return;
                }
                // 僵尸面向玩家才发起攻击
                boomer.call_deferred("dying", &[]);
                if let Some(mut tree) = self.base().get_tree() {
                    if let Some(mut timer) =
                        tree.create_timer(boomer.bind().get_explode_countdown() as f64)
                    {
                        timer.connect("timeout", &boomer.callable("die"));
                    }
                }
            }
        }
    }

    fn get_parent(&self) -> Gd<Node> {
        self.base()
            .get_parent()
            .expect("ZombieExplodeArea parent not found")
    }
}
