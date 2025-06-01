use crate::is_survivor;
use crate::zombie::pitcher::RustPitcher;
use godot::classes::{Area2D, IArea2D, Node, Node2D, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct ZombiePitchArea {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for ZombiePitchArea {
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
impl ZombiePitchArea {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        if is_survivor(&***body) {
            if let Ok(mut pitcher) = self.get_parent().try_cast::<RustPitcher>() {
                if !pitcher.bind().is_face_to_user() {
                    return;
                }
                // 僵尸面向玩家才发起攻击
                pitcher.bind_mut().attack();
                pitcher.bind_mut().set_attacking(true);
            }
        }
    }

    #[func]
    pub fn on_area_2d_body_exited(&mut self, body: Gd<Node2D>) {
        if is_survivor(&***body) {
            if let Ok(mut pitcher) = self.get_parent().try_cast::<RustPitcher>() {
                pitcher.bind_mut().guard();
                pitcher.bind_mut().set_attacking(false);
            }
        }
    }

    fn get_parent(&self) -> Gd<Node> {
        self.base()
            .get_parent()
            .expect("ZombiePitchArea parent not found")
    }
}
