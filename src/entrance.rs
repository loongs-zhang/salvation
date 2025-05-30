use crate::scale_rate;
use crate::world::RustWorld;
use godot::classes::{
    AudioStreamPlayer2D, Button, ColorRect, Control, IControl, PackedScene, Tween, VBoxContainer,
};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Control)]
pub struct RustEntrance {
    world_scene: OnReady<Gd<PackedScene>>,
    bgm: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Control>,
}

#[godot_api]
impl IControl for RustEntrance {
    fn init(base: Base<Control>) -> Self {
        Self {
            world_scene: OnReady::from_loaded("res://scenes/world/rust_world.tscn"),
            bgm: OnReady::from_node("Bgm"),
            base,
        }
    }

    fn enter_tree(&mut self) {
        self.scale();
    }

    fn ready(&mut self) {
        let gd = self.to_gd();
        let container = self.base().get_node_as::<VBoxContainer>("VBoxContainer");
        container
            .get_node_as::<Button>("Load")
            .signals()
            .pressed()
            .connect_obj(&gd, Self::on_load_pressed);
        container
            .get_node_as::<Button>("HellMode")
            .signals()
            .pressed()
            .connect_obj(&gd, Self::on_hell_mode_pressed);
        container
            .get_node_as::<Button>("EndlessMode")
            .signals()
            .pressed()
            .connect_obj(&gd, Self::on_endless_mode_pressed);
        container
            .get_node_as::<Button>("ExitGame")
            .signals()
            .pressed()
            .connect_obj(&gd, Self::on_exit_game_pressed);
        self.play_bgm();
        self.bgm
            .signals()
            .finished()
            .connect_obj(&gd, Self::play_bgm);
    }
}

#[godot_api]
impl RustEntrance {
    pub fn scale(&self) {
        self.base()
            .get_window()
            .unwrap()
            .set_content_scale_factor(scale_rate());
    }

    #[func]
    pub fn play_bgm(&mut self) {
        self.bgm.play();
    }

    #[func]
    pub fn on_load_pressed(&mut self) {
        self.prepare().tween_callback(
            &self
                .base_mut()
                .callable("change_scene")
                .bind(&[false.to_variant(), true.to_variant()]),
        );
    }

    #[func]
    pub fn on_hell_mode_pressed(&mut self) {
        self.prepare().tween_callback(
            &self
                .base_mut()
                .callable("change_scene")
                .bind(&[true.to_variant(), false.to_variant()]),
        );
    }

    fn prepare(&mut self) -> Gd<Tween> {
        let container = self.base().get_node_as::<VBoxContainer>("VBoxContainer");
        container.get_node_as::<Button>("Load").set_visible(false);
        container
            .get_node_as::<Button>("HellMode")
            .set_visible(false);
        container
            .get_node_as::<Button>("EndlessMode")
            .set_visible(false);
        container
            .get_node_as::<Button>("ExitGame")
            .set_visible(false);
        let color_rect = container
            .get_parent()
            .expect("RustEntrance not found")
            .get_node_as::<ColorRect>("ColorRect");
        let mut tween = self
            .base_mut()
            .create_tween()
            .expect("Failed to create tween");
        tween
            .tween_property(&color_rect, "modulate:a", &1_f32.to_variant(), 1.0)
            .unwrap()
            .from(&0_f32.to_variant());
        tween
    }

    #[func]
    pub fn on_endless_mode_pressed(&mut self) {
        self.prepare().tween_callback(
            &self
                .base_mut()
                .callable("change_scene")
                .bind(&[false.to_variant(), false.to_variant()]),
        );
    }

    #[func]
    pub fn change_scene(&mut self, hell: bool, load: bool) {
        if let Some(mut world) = self.world_scene.try_instantiate_as::<RustWorld>() {
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    world.bind_mut().set_hell(hell);
                    world.bind_mut().set_load(load);
                    root.add_child(&world);
                }
            }
        }
    }

    #[func]
    pub fn on_exit_game_pressed(&mut self) {
        if let Some(mut tree) = self.base().get_tree() {
            tree.quit();
        }
    }
}
