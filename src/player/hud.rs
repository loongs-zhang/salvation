use crate::player::RustPlayer;
use godot::builtin::real;
use godot::classes::input::MouseMode;
use godot::classes::notify::NodeNotification;
use godot::classes::{
    Button, CanvasLayer, Control, HBoxContainer, ICanvasLayer, Input, Label, TextureRect,
    VBoxContainer,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=CanvasLayer)]
pub struct PlayerHUD {
    cross_hair: OnReady<Gd<TextureRect>>,
    control: OnReady<Gd<Control>>,
    upgrade: OnReady<Gd<Control>>,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for PlayerHUD {
    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            cross_hair: OnReady::from_node("CrossHair"),
            control: OnReady::from_node("Control"),
            upgrade: OnReady::from_node("Upgrade"),
            base,
        }
    }

    fn ready(&mut self) {
        Input::singleton().set_mouse_mode(MouseMode::HIDDEN);
        if let Some(parent) = self.base().get_parent() {
            let player = parent.cast::<RustPlayer>();
            self.get_container()
                .get_node_as::<Button>("Penetrate")
                .signals()
                .pressed()
                .connect_obj(&player, RustPlayer::upgrade_penetrate);
            self.get_container()
                .get_node_as::<Button>("Damage")
                .signals()
                .pressed()
                .connect_obj(&player, RustPlayer::upgrade_damage);
            self.get_container()
                .get_node_as::<Button>("Repel")
                .signals()
                .pressed()
                .connect_obj(&player, RustPlayer::upgrade_repel);
            self.get_container()
                .get_node_as::<Button>("Lives")
                .signals()
                .pressed()
                .connect_obj(&player, RustPlayer::upgrade_lives);
            self.get_container()
                .get_node_as::<Button>("Distance")
                .signals()
                .pressed()
                .connect_obj(&player, RustPlayer::upgrade_distance);
            self.get_container()
                .get_node_as::<Button>("Health")
                .signals()
                .pressed()
                .connect_obj(&player, RustPlayer::upgrade_health);
        }
    }

    fn process(&mut self, _delta: f64) {
        let viewport = self.base().get_viewport().unwrap();
        let affine_inverse = self.base().get_transform().affine_inverse();
        let mouse_position =
            affine_inverse * viewport.get_mouse_position() - self.cross_hair.get_size() / 2.0;
        self.cross_hair.set_position(mouse_position);
    }

    fn on_notification(&mut self, what: NodeNotification) {
        match what {
            NodeNotification::WM_MOUSE_ENTER => {
                //鼠标进入窗口时隐藏
                Input::singleton().set_mouse_mode(MouseMode::HIDDEN);
            }
            NodeNotification::WM_MOUSE_EXIT | NodeNotification::WM_WINDOW_FOCUS_OUT => {
                //鼠标离开窗口或窗口失去焦点时显示
                Input::singleton().set_mouse_mode(MouseMode::VISIBLE);
            }
            _ => {}
        }
    }

    fn exit_tree(&mut self) {
        Input::singleton().set_mouse_mode(MouseMode::VISIBLE);
    }
}

#[godot_api]
impl PlayerHUD {
    pub fn set_upgrade_visible(&mut self, visible: bool) {
        self.upgrade.set_visible(visible);
    }

    pub fn update_lives_hud(&mut self, lives: u32, max_lives: u32) {
        let mut hp_hud = self.get_vcontainer().get_node_as::<Label>("Lives");
        hp_hud.set_text(&format!("LIVES {}/{}", lives, max_lives));
        hp_hud.show();
    }

    pub fn update_hp_hud(&mut self, hp: u32, max_hp: u32) {
        let mut hp_hud = self.get_vcontainer().get_node_as::<Label>("HP");
        hp_hud.set_text(&format!("HP {}/{}", hp, max_hp));
        hp_hud.show();
    }

    pub fn update_ammo_hud(&mut self, ammo: i64, clip: i64) {
        let mut ammo_hud = self.get_vcontainer().get_node_as::<Label>("Ammo");
        ammo_hud.set_text(&format!("AMMO {}/{}", ammo, clip));
        ammo_hud.show();
    }

    pub fn update_damage_hud(&mut self, damage: i64) {
        let mut damage_hud = self.get_vcontainer().get_node_as::<Label>("Damage");
        damage_hud.set_text(&format!("DAMAGE {}", damage));
        damage_hud.show();
    }

    pub fn update_distance_hud(&mut self, distance: real) {
        let mut damage_hud = self.get_vcontainer().get_node_as::<Label>("Distance");
        damage_hud.set_text(&format!("DISTANCE {:.0}", distance));
        damage_hud.show();
    }

    pub fn update_penetrate_hud(&mut self, penetrate: real) {
        let mut penetrate_hud = self.get_vcontainer().get_node_as::<Label>("Penetrate");
        penetrate_hud.set_text(&format!("PENETRATE {:.1}", penetrate));
        penetrate_hud.show();
    }

    pub fn update_repel_hud(&mut self, repel: real) {
        let mut repel_hud = self.get_vcontainer().get_node_as::<Label>("Repel");
        repel_hud.set_text(&format!("REPEL {}", repel));
        repel_hud.show();
    }

    pub fn update_killed_hud(&mut self) {
        let mut repel_hud = self.get_hcontainer().get_node_as::<Label>("Killed");
        repel_hud.set_text(&format!(
            "KILLED {}+{}",
            RustPlayer::get_kill_boss_count(),
            RustPlayer::get_kill_count()
        ));
        repel_hud.show();
    }

    pub fn update_score_hud(&mut self) {
        let mut repel_hud = self.get_hcontainer().get_node_as::<Label>("Score");
        repel_hud.set_text(&format!("SCORE {}", RustPlayer::get_score()));
        repel_hud.show();
    }

    fn get_vcontainer(&mut self) -> Gd<VBoxContainer> {
        self.control.get_node_as::<VBoxContainer>("VBoxContainer")
    }

    fn get_hcontainer(&mut self) -> Gd<HBoxContainer> {
        self.control.get_node_as::<HBoxContainer>("HBoxContainer")
    }

    fn get_container(&mut self) -> Gd<VBoxContainer> {
        self.upgrade.get_node_as::<VBoxContainer>("VBoxContainer")
    }
}
