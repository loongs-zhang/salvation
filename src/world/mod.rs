use crate::PlayerState;
use crate::entrance::RustEntrance;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::ground::RustGround;
use godot::builtin::{Vector2, Vector2i, real};
use godot::classes::{
    Button, CanvasLayer, Control, HBoxContainer, INode2D, InputEvent, Label, Node, Node2D, Object,
    PackedScene,
};
use godot::global::godot_print;
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use godot::tools::load;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Instant;

pub mod ground;

const RANGE: i32 = 256;

const STEP: i32 = 8;

const STEP_VECTOR2I: Vector2i = Vector2i::new(STEP, STEP);

static MAX_LEFT: AtomicI32 = AtomicI32::new(0);

static MAX_RIGHT: AtomicI32 = AtomicI32::new(0);

static MAX_TOP: AtomicI32 = AtomicI32::new(0);

static MAX_BOTTOM: AtomicI32 = AtomicI32::new(0);

static PAUSED: AtomicBool = AtomicBool::new(false);

#[allow(clippy::declare_interior_mutable_const)]
const GROUND: LazyLock<Gd<PackedScene>> = LazyLock::new(|| load("res://scenes/rust_ground.tscn"));

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWorld {
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
        self.generate_world();
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

    #[func]
    pub fn generate_world(&mut self) {
        let player_position = self.rust_player.get_global_position();
        if !Self::can_generate(player_position) {
            return;
        }
        let now = Instant::now();
        for i in (-RANGE..RANGE).step_by(STEP as usize) {
            for j in (-RANGE..RANGE).step_by(STEP as usize) {
                let from =
                    Vector2i::new(player_position.x as i32 + i, player_position.y as i32 + j);
                let to = from + STEP_VECTOR2I;
                MAX_LEFT.store(
                    MAX_LEFT.load(Ordering::Acquire).max(to.x),
                    Ordering::Release,
                );
                MAX_RIGHT.store(
                    MAX_RIGHT.load(Ordering::Acquire).min(from.x),
                    Ordering::Release,
                );
                MAX_TOP.store(MAX_TOP.load(Ordering::Acquire).max(to.y), Ordering::Release);
                MAX_BOTTOM.store(
                    MAX_BOTTOM.load(Ordering::Acquire).min(from.y),
                    Ordering::Release,
                );
                if !Self::can_generate(player_position) {
                    continue;
                }
                #[allow(clippy::borrow_interior_mutable_const)]
                if let Some(mut ground) = GROUND.try_instantiate_as::<RustGround>() {
                    ground.bind_mut().set_from(from);
                    ground.bind_mut().set_to(to);
                    self.base_mut().add_child(&ground);
                }
            }
        }
        godot_print!(
            "Generated world cost {}ms",
            Instant::now().duration_since(now).as_millis()
        );
    }

    fn can_generate(player_position: Vector2) -> bool {
        player_position.x - RANGE as real <= MAX_LEFT.load(Ordering::Acquire) as real
            || player_position.x + (RANGE as real) >= MAX_RIGHT.load(Ordering::Acquire) as real
            || player_position.y - RANGE as real <= MAX_TOP.load(Ordering::Acquire) as real
            || player_position.y + (RANGE as real) >= MAX_BOTTOM.load(Ordering::Acquire) as real
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
