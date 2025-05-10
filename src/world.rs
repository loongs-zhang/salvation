use crate::entrance::RustEntrance;
use crate::player::RustPlayer;
use godot::builtin::{Array, Vector2, Vector2i, array, real};
use godot::classes::fast_noise_lite::NoiseType;
use godot::classes::{
    Button, CanvasLayer, Control, FastNoiseLite, HBoxContainer, INode2D, InputEvent, Node2D,
    Object, PackedScene, TileMapLayer,
};
use godot::global::godot_print;
use godot::obj::{Base, Gd, NewGd, OnReady, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use rand::Rng;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

const SOIL_TERRAIN_SET: i32 = 0;
const SAND_TERRAIN_SET: i32 = 1;
const GLASS_TERRAIN_SET: i32 = 2;
const SOURCE_ID: i32 = 0;

static PAUSED: AtomicBool = AtomicBool::new(false);

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWorld {
    entrance_scene: OnReady<Gd<PackedScene>>,
    tile_map_layer: OnReady<Gd<TileMapLayer>>,
    rust_player: OnReady<Gd<RustPlayer>>,
    game_over: OnReady<Gd<CanvasLayer>>,
    generated: HashSet<Vector2i>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWorld {
    fn init(base: Base<Node2D>) -> Self {
        // We could also initialize those manually inside ready(), but OnReady automatically defers initialization.
        // Alternatively to init(), you can use #[init(...)] on the struct fields.
        Self {
            entrance_scene: OnReady::from_loaded("res://scenes/rust_entrance.tscn"),
            tile_map_layer: OnReady::from_node("TileMapLayer"),
            rust_player: OnReady::from_node("RustPlayer"),
            game_over: OnReady::from_node("CanvasLayer"),
            generated: HashSet::new(),
            base,
        }
    }

    fn ready(&mut self) {
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
        self.generate_world(125);
        // stop BGM after world generated
        self.base()
            .get_tree()
            .unwrap()
            .get_root()
            .unwrap()
            .get_node_as::<RustEntrance>("RustEntrance")
            .queue_free();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("p") {
            Self::pause();
        }
    }
}

#[godot_api]
impl RustWorld {
    #[signal]
    pub fn player_dead();

    #[func]
    pub fn on_player_dead(&mut self) {
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
        self.rust_player.bind_mut().reborn();
    }

    #[func]
    pub fn generate(&mut self) {
        self.generate_world(20);
    }

    pub fn generate_world(&mut self, square_half_length: i32) {
        let now = Instant::now();
        let player = self.rust_player.get_global_position();
        let glass_atlas: Array<Vector2i> = array![
            Vector2i::new(0, 0),
            Vector2i::new(1, 0),
            Vector2i::new(2, 0),
            Vector2i::new(3, 0),
            Vector2i::new(4, 0)
        ];
        let mut soil_array = Array::new();
        let mut sand_array = Array::new();
        let mut glass_array = Array::new();

        let mut noise = FastNoiseLite::new_gd();
        noise.set_noise_type(NoiseType::SIMPLEX_SMOOTH);
        noise.set_seed(rand::random::<i32>());
        noise.set_frequency(0.08);
        // generate world
        for x in player.x as i32 - square_half_length..player.x as i32 + square_half_length {
            for y in player.y as i32 - square_half_length..player.y as i32 + square_half_length {
                let vector2i = Vector2i::new(x, y);
                if self.generated.contains(&vector2i) {
                    continue;
                }
                let val = noise.get_noise_2d(x as f32, y as f32);
                if val <= 0.0 {
                    soil_array.push(vector2i);
                } else if val <= 0.1 {
                    sand_array.push(vector2i);
                } else if val <= 0.2 {
                    self.tile_map_layer
                        .set_cell_ex(vector2i)
                        .source_id(SOURCE_ID)
                        .atlas_coords(
                            glass_atlas
                                .pick_random()
                                .expect("Atlas should not be empty"),
                        )
                        .done();
                } else if val <= 0.4 {
                    glass_array.push(vector2i);
                } else {
                    soil_array.push(vector2i);
                }
                self.generated.insert(vector2i);
            }
        }
        if !soil_array.is_empty() {
            self.tile_map_layer
                .set_cells_terrain_connect(&soil_array, SOIL_TERRAIN_SET, 0);
        }
        if !sand_array.is_empty() {
            self.tile_map_layer
                .set_cells_terrain_connect(&sand_array, SAND_TERRAIN_SET, 0);
        }
        if !glass_array.is_empty() {
            self.tile_map_layer
                .set_cells_terrain_connect(&glass_array, GLASS_TERRAIN_SET, 0);
        }
        godot_print!(
            "Generated {} with {} soil, {} sand and {} glass tiles, cost {}ms",
            self.base.to_gd(),
            soil_array.len(),
            sand_array.len(),
            glass_array.len(),
            Instant::now().duration_since(now).as_millis()
        );
    }

    pub fn random_position() -> Vector2 {
        Vector2::new(Self::random_half_position(), Self::random_half_position())
    }

    fn random_half_position() -> real {
        let mut rng = rand::thread_rng();
        if rng.gen_range(-1.0..1.0) >= 0.0 {
            rng.gen_range(275.0..500.0)
        } else {
            rng.gen_range(-500.0..-275.0)
        }
    }

    pub fn pause() {
        if PAUSED.load(Ordering::Acquire) {
            PAUSED.store(false, Ordering::Release);
        } else {
            PAUSED.store(true, Ordering::Release);
        }
    }

    pub fn is_paused() -> bool {
        PAUSED.load(Ordering::Acquire)
    }
}
