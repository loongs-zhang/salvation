use crate::entrance::RustEntrance;
use crate::level::RustLevel;
use crate::player::RustPlayer;
use crate::world::ground::RustGround;
use crate::{PlayerState, scale_rate};
use dashmap::DashSet;
use godot::builtin::{Array, Vector2, Vector2i};
use godot::classes::{
    Button, CanvasLayer, Control, HBoxContainer, INode2D, Input, InputEvent, Label, Node, Node2D,
    Object, PackedScene, Timer,
};
use godot::global::godot_print;
use godot::obj::{Base, Gd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use godot::tools::load;
use std::ops::Range;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Instant;

pub mod ground;

const STEP: i32 = 10;

const STEP_VECTOR2I: Vector2i = Vector2i::new(STEP, STEP);

static GENERATED: LazyLock<DashSet<Vector2i>> = LazyLock::new(DashSet::new);

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
    timer: OnReady<Gd<Timer>>,
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
            timer: OnReady::from_node("Timer"),
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
        // todo 继续优化无限地图
        // let callable = self.base().callable("grow_world");
        // self.timer.connect("timeout", &callable);
        // self.timer.set_wait_time(0.125);
        // self.timer.set_one_shot(true);
        // self.timer.start();
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
        let rect2 = self.rust_player.get_viewport_rect();
        let size = (rect2.size / scale_rate() / 1.5).cast_int();
        let position = player_position.cast_int() + rect2.position.cast_int() - size / 2;
        let now = Instant::now();
        for x in position.x..position.x + size.x {
            if 0 != x % STEP {
                continue;
            }
            for y in position.y..position.y + size.y {
                if 0 != y % STEP {
                    continue;
                }
                let from = Vector2i::new(x, y);
                let to = from + STEP_VECTOR2I;
                let points = Self::find_points(from, to);
                if points.is_empty() {
                    continue;
                }
                for point in points.iter_shared() {
                    GENERATED.insert(point);
                    MAX_LEFT.store(
                        MAX_LEFT.load(Ordering::Acquire).max(point.x),
                        Ordering::Release,
                    );
                    MAX_RIGHT.store(
                        MAX_RIGHT.load(Ordering::Acquire).min(point.x),
                        Ordering::Release,
                    );
                    MAX_TOP.store(
                        MAX_TOP.load(Ordering::Acquire).max(point.y),
                        Ordering::Release,
                    );
                    MAX_BOTTOM.store(
                        MAX_BOTTOM.load(Ordering::Acquire).min(point.y),
                        Ordering::Release,
                    );
                }
                self.install_ground(points);
            }
        }
        godot_print!(
            "Generated world cost {}ms {} {}",
            Instant::now().duration_since(now).as_millis(),
            position,
            position + size,
        );
    }

    #[func]
    pub fn grow_world(&mut self) {
        let now = Instant::now();
        let player_position = self.rust_player.get_global_position().cast_int();
        let rect2 = self.rust_player.get_viewport_rect();
        let size = (rect2.size / scale_rate() / 1.5).cast_int();
        let position = player_position + rect2.position.cast_int() - size / 2;
        let input = Input::singleton();
        if input.is_action_pressed("move_left") {
            let next = MAX_LEFT.load(Ordering::Acquire) - STEP;
            if position.x < next {
                self.install_ground(Self::build_points(
                    next..next + STEP,
                    position.y..position.y + size.y,
                ));
                MAX_LEFT.store(next, Ordering::Release);
            }
        } else if input.is_action_pressed("move_right") {
            let next = MAX_RIGHT.load(Ordering::Acquire) + STEP;
            if position.x > next {
                self.install_ground(Self::build_points(
                    next - STEP..next,
                    position.y..position.y + size.y,
                ));
                MAX_RIGHT.store(next, Ordering::Release);
            }
        }
        if input.is_action_pressed("move_up") {
            let next = MAX_TOP.load(Ordering::Acquire) - STEP;
            if position.y < next {
                self.install_ground(Self::build_points(
                    position.x..position.x + size.x,
                    next..next + STEP,
                ));
                MAX_TOP.store(next, Ordering::Release);
            }
        } else if input.is_action_pressed("move_down") {
            let next = MAX_BOTTOM.load(Ordering::Acquire) + STEP;
            if position.y > next {
                self.install_ground(Self::build_points(
                    position.x..position.x + size.x,
                    next - STEP..next,
                ));
                MAX_BOTTOM.store(next, Ordering::Release);
            }
        }
        self.base_mut().queue_redraw();
        godot_print!(
            "Grow world cost {}ms {} {}",
            Instant::now().duration_since(now).as_millis(),
            position,
            position + size,
        );
        self.timer.start();
    }

    fn install_ground(&mut self, points: Array<Vector2i>) {
        if points.is_empty() {
            return;
        }
        #[allow(clippy::borrow_interior_mutable_const)]
        if let Some(mut ground) = GROUND.try_instantiate_as::<RustGround>() {
            ground.bind_mut().set_points(points);
            self.base_mut().add_child(&ground);
        }
    }

    fn build_points(x: Range<i32>, y: Range<i32>) -> Array<Vector2i> {
        let mut points = Array::new();
        for x in x {
            for y in y.clone() {
                let point = Vector2i::new(x, y);
                if GENERATED.contains(&point) {
                    continue;
                }
                points.push(point);
                GENERATED.insert(point);
            }
        }
        points
    }

    fn find_points(from: Vector2i, to: Vector2i) -> Array<Vector2i> {
        let mut points = Array::new();
        for x in from.x..to.x {
            for y in from.y..to.y {
                let vector2i = Vector2i::new(x, y);
                if GENERATED.contains(&vector2i) {
                    return Array::new();
                }
                points.push(vector2i);
            }
        }
        points
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
