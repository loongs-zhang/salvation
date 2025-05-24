use crate::PlayerState;
use crate::entrance::RustEntrance;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use godot::builtin::Vector2;
use godot::classes::{
    Button, CanvasLayer, Control, HBoxContainer, INode2D, InputEvent, Label, Node, Node2D, Object,
    PackedScene,
};
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use std::sync::atomic::{AtomicBool, Ordering};

pub mod ground;

static PAUSED: AtomicBool = AtomicBool::new(false);

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWorld {
    #[export]
    hell: bool,
    entrance_scene: OnReady<Gd<PackedScene>>,
    rust_player: OnReady<Gd<RustPlayer>>,
    rust_level: OnReady<Gd<RustLevel>>,
    game_over: OnReady<Gd<CanvasLayer>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWorld {
    fn init(base: Base<Node2D>) -> Self {
        // We could also initialize those manually inside ready(), but OnReady automatically defers initialization.
        // Alternatively to init(), you can use #[init(...)] on the struct fields.
        Self {
            hell: false,
            entrance_scene: OnReady::from_loaded("res://scenes/rust_entrance.tscn"),
            rust_player: OnReady::from_node("RustPlayer"),
            rust_level: OnReady::from_node("RustLevel"),
            game_over: OnReady::from_node("CanvasLayer"),
            base,
        }
    }

    fn ready(&mut self) {
        if Self::is_paused() {
            Self::resume();
        }
        let gd = self.to_gd();
        let container = self
            .game_over
            .get_node_as::<Control>("Control")
            .get_node_as::<HBoxContainer>("HBoxContainer");
        container
            .get_node_as::<Button>("Exit")
            .signals()
            .pressed()
            .connect_obj(&gd, Self::on_exit_pressed);
        container
            .get_node_as::<Button>("Continue")
            .signals()
            .pressed()
            .connect_obj(&gd, Self::on_continue_pressed);
        self.signals()
            .player_dead()
            .connect_self(Self::on_player_dead);
        // stop BGM after world generated
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                if let Some(mut entrance) = root.try_get_node_as::<Node>("RustEntrance") {
                    entrance.queue_free();
                }
            }
        }
        if self.hell {
            let mut rust_level = self.rust_level.bind_mut();
            rust_level.set_rampage_time(0.0);
            rust_level.set_zombie_refresh_time(0.2);
            rust_level.set_boss_refresh_time(0.2);
            rust_level.refresh();
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("esc") {
            if self.game_over.is_visible() {
                self.game_over.set_visible(false);
                Self::resume();
            } else {
                let mut message = self
                    .game_over
                    .get_node_as::<Control>("Control")
                    .get_node_as::<Label>("Message");
                message.set_text("Game paused");
                message.show();
                self.game_over.set_visible(true);
                Self::pause();
            }
        }
    }
}

#[godot_api]
impl RustWorld {
    #[signal]
    pub fn player_dead();

    #[func]
    pub fn on_player_dead(&mut self) {
        let mut message = self
            .game_over
            .get_node_as::<Control>("Control")
            .get_node_as::<Label>("Message");
        message.set_text("You have turned");
        message.show();
        self.game_over.set_visible(true);
    }

    #[func]
    pub fn on_exit_pressed(&mut self) {
        if let Some(world) = self.entrance_scene.try_instantiate_as::<RustEntrance>() {
            if let Some(tree) = self.base().get_tree() {
                if let Some(mut root) = tree.get_root() {
                    root.add_child(&world);
                    self.base_mut().queue_free();
                }
            }
        }
    }

    #[func]
    pub fn on_continue_pressed(&mut self) {
        self.game_over.set_visible(false);
        if PlayerState::Dead == RustPlayer::get_state() {
            self.rust_player.bind_mut().reborn();
        }
        self.rust_level.bind_mut().start();
        if Self::is_paused() {
            Self::resume();
        }
    }

    pub fn random_position() -> Vector2 {
        crate::random_position(275.0, 500.0)
    }

    pub fn pause() {
        PAUSED.store(true, Ordering::Release);
    }

    pub fn resume() {
        PAUSED.store(false, Ordering::Release);
    }

    pub fn is_paused() -> bool {
        PAUSED.load(Ordering::Acquire)
    }
}
