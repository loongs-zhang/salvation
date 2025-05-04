use crate::world::RustWorld;
use godot::classes::{Button, Control, IButton, IControl, Object, PackedScene, VBoxContainer};
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Control)]
pub struct RustEntrance {
    base: Base<Control>,
}

#[godot_api]
impl IControl for RustEntrance {
    fn init(base: Base<Control>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        let container = self.base().get_node_as::<VBoxContainer>("VBoxContainer");
        container
            .get_node_as::<EndlessMode>("EndlessMode")
            .signals()
            .pressed()
            .connect_self(EndlessMode::on_endless_mode_pressed);
        container
            .get_node_as::<ExitGame>("ExitGame")
            .signals()
            .pressed()
            .connect_self(ExitGame::on_exit_game_pressed);
    }
}

#[derive(GodotClass)]
#[class(base=Button)]
pub struct EndlessMode {
    world_scene: OnReady<Gd<PackedScene>>,
    base: Base<Button>,
}

#[godot_api]
impl IButton for EndlessMode {
    fn init(base: Base<Button>) -> Self {
        Self {
            world_scene: OnReady::from_loaded("res://scenes/rust_world.tscn"),
            base,
        }
    }
}

#[godot_api]
impl EndlessMode {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_endless_mode_pressed(&mut self) {
        if let Some(world) = self.world_scene.try_instantiate_as::<RustWorld>() {
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&world);
                }
            }
        }
    }
}

#[derive(GodotClass)]
#[class(base=Button)]
pub struct ExitGame {
    base: Base<Button>,
}

#[godot_api]
impl IButton for ExitGame {
    fn init(base: Base<Button>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl ExitGame {
    #[signal]
    pub fn sig();

    #[func]
    pub fn on_exit_game_pressed(&mut self) {
        if let Some(mut tree) = self.base().get_tree() {
            tree.quit();
        }
    }
}
