use godot::classes::{CanvasLayer, Control, ICanvasLayer, Label, VBoxContainer};
use godot::obj::{Base, Gd, OnReady};
use godot::prelude::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=CanvasLayer)]
pub struct WeaponHUD {
    control: OnReady<Gd<Control>>,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for WeaponHUD {
    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            control: OnReady::from_node("Control"),
            base,
        }
    }
}

#[godot_api]
impl WeaponHUD {
    pub fn update_ammo_hud(&mut self, ammo: i64, clip: i64) {
        let mut ammo_hud = self
            .control
            .get_node_as::<VBoxContainer>("VBoxContainer")
            .get_node_as::<Label>("Ammo");
        ammo_hud.set_text(&format!("AMMO {}/{}", ammo, clip));
        ammo_hud.show();
    }
}
