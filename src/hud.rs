use crate::player::RustPlayer;
use godot::builtin::real;
use godot::classes::input::MouseMode;
use godot::classes::notify::NodeNotification;
use godot::classes::{
    Button, CanvasLayer, Control, Engine, HBoxContainer, ICanvasLayer, Input, Label, TextureRect,
    VBoxContainer,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::{GodotClass, godot_api};

// todo 合并武器的HUD

#[derive(GodotClass)]
#[class(base=CanvasLayer)]
pub struct RustHUD {
    cross_hair: OnReady<Gd<TextureRect>>,
    control: OnReady<Gd<Control>>,
    upgrade: OnReady<Gd<Control>>,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for RustHUD {
    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            cross_hair: OnReady::from_node("CrossHair"),
            control: OnReady::from_node("Control"),
            upgrade: OnReady::from_node("Upgrade"),
            base,
        }
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

    fn process(&mut self, _delta: f64) {
        let viewport = self.base().get_viewport().unwrap();
        let affine_inverse = self.base().get_transform().affine_inverse();
        let mouse_position =
            affine_inverse * viewport.get_mouse_position() - self.cross_hair.get_size() / 2.0;
        self.cross_hair.set_position(mouse_position);
    }

    fn exit_tree(&mut self) {
        Input::singleton().set_mouse_mode(MouseMode::VISIBLE);
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
}

#[godot_api]
impl RustHUD {
    pub fn update_lives_hud(&mut self, lives: u32, max_lives: u32) {
        let mut hp_hud = self.get_left_container().get_node_as::<Label>("Lives");
        hp_hud.set_text(&format!("LIVES {}/{}", lives, max_lives));
        hp_hud.show();
    }

    pub fn update_hp_hud(&mut self, hp: u32, max_hp: u32) {
        let mut hp_hud = self.get_left_container().get_node_as::<Label>("HP");
        hp_hud.set_text(&format!("HP {}/{}", hp, max_hp));
        hp_hud.show();
    }

    pub fn update_damage_hud(&mut self, damage: i64) {
        let mut damage_hud = self.get_left_container().get_node_as::<Label>("Damage");
        damage_hud.set_text(&format!("DAMAGE {}", damage));
        damage_hud.show();
    }

    pub fn update_distance_hud(&mut self, distance: real) {
        let mut damage_hud = self.get_left_container().get_node_as::<Label>("Distance");
        damage_hud.set_text(&format!("DISTANCE {:.0}", distance));
        damage_hud.show();
    }

    pub fn update_penetrate_hud(&mut self, penetrate: real) {
        let mut penetrate_hud = self.get_left_container().get_node_as::<Label>("Penetrate");
        penetrate_hud.set_text(&format!("PENETRATE {:.1}", penetrate));
        penetrate_hud.show();
    }

    pub fn update_repel_hud(&mut self, repel: real) {
        let mut repel_hud = self.get_left_container().get_node_as::<Label>("Repel");
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

    pub fn update_died_hud(&mut self) {
        let mut repel_hud = self.get_hcontainer().get_node_as::<Label>("Died");
        repel_hud.set_text(&format!("DIED {}", RustPlayer::get_died()));
        repel_hud.show();
    }

    pub fn update_level_hud(&mut self, level: u32) {
        let mut label = self.get_center_container().get_node_as::<Label>("Level");
        label.set_text(&format!("LEVEL {}", level));
        label.show();
    }

    pub fn update_rampage_hud(&mut self, left_rampage_time: real) {
        let mut label = self.get_center_container().get_node_as::<Label>("Rampage");
        label.set_text(&format!("ZOMBIE RAMPAGE {:.1} s", left_rampage_time));
        label.show();
    }

    pub fn update_progress_hud(
        &mut self,
        boss_killed: u32,
        zombie_killed: u32,
        boss_refreshed: u32,
        zombie_refreshed: u32,
        boss_total: u32,
        zombie_total: u32,
    ) {
        let mut label = self.get_center_container().get_node_as::<Label>("Progress");
        label.set_text(&format!(
            "PROGRESS {}+{}/{}+{}/{}+{}",
            boss_killed, zombie_killed, boss_refreshed, zombie_refreshed, boss_total, zombie_total
        ));
        label.show();
    }

    pub fn update_refresh_zombie_hud(
        &mut self,
        is_stopped: bool,
        zombie_refresh_count: u32,
        zombie_wait_time: f64,
    ) {
        let mut label = self
            .get_right_container()
            .get_node_as::<Label>("RefreshZombie");
        label.set_text(&format!(
            "ZOMBIE {} {}/{:.1}s",
            if is_stopped { "COMING" } else { "INCOMING" },
            zombie_refresh_count,
            zombie_wait_time,
        ));
        label.show();
    }

    pub fn update_refresh_boomer_hud(
        &mut self,
        is_stopped: bool,
        boomer_refresh_count: u32,
        boomer_wait_time: f64,
    ) {
        let mut label = self
            .get_right_container()
            .get_node_as::<Label>("RefreshBoomer");
        label.set_text(&format!(
            "BOOMER {} {}/{:.1}s",
            if is_stopped { "COMING" } else { "INCOMING" },
            boomer_refresh_count,
            boomer_wait_time,
        ));
        label.show();
    }

    pub fn update_refresh_boss_hud(
        &mut self,
        is_stopped: bool,
        boss_refresh_count: u32,
        boss_wait_time: f64,
    ) {
        let mut label = self
            .get_right_container()
            .get_node_as::<Label>("RefreshBoss");
        label.set_text(&format!(
            "BOSS {} {}/{:.1}s",
            if is_stopped { "COMING" } else { "INCOMING" },
            boss_refresh_count,
            boss_wait_time,
        ));
        label.show();
    }

    pub fn update_fps_hud(&mut self) {
        let mut label = self.get_right_container().get_node_as::<Label>("FPS");
        label.set_text(&format!(
            "FPS {}",
            Engine::singleton().get_frames_per_second(),
        ));
        label.show();
    }

    pub fn set_upgrade_visible(&mut self, visible: bool) {
        self.upgrade.set_visible(visible);
    }

    fn get_left_container(&mut self) -> Gd<VBoxContainer> {
        self.control.get_node_as::<VBoxContainer>("VBoxTopLeft")
    }

    fn get_center_container(&mut self) -> Gd<VBoxContainer> {
        self.control.get_node_as::<VBoxContainer>("VBoxTopCenter")
    }

    fn get_right_container(&mut self) -> Gd<VBoxContainer> {
        self.control.get_node_as::<VBoxContainer>("VBoxTopRight")
    }

    fn get_container(&mut self) -> Gd<VBoxContainer> {
        self.upgrade.get_node_as::<VBoxContainer>("VBoxContainer")
    }

    fn get_hcontainer(&mut self) -> Gd<HBoxContainer> {
        self.get_center_container()
            .get_node_as::<HBoxContainer>("HBoxContainer")
    }
}
