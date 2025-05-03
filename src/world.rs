use crate::player::RustPlayer;
use crate::zombie::RustZombie;
use godot::builtin::{Array, Vector2, Vector2i, array};
use godot::classes::fast_noise_lite::NoiseType;
use godot::classes::{FastNoiseLite, INode2D, Node2D, PackedScene, TileMapLayer};
use godot::global::godot_print;
use godot::obj::{Base, Gd, NewGd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use std::collections::HashSet;
use std::time::Instant;

const SOIL_TERRAIN_SET: i32 = 0;
const SAND_TERRAIN_SET: i32 = 1;
const GLASS_TERRAIN_SET: i32 = 2;
const SOURCE_ID: i32 = 0;

// Deriving GodotClass makes the class available to Godot.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWorld {
    tile_map_layer: OnReady<Gd<TileMapLayer>>,
    rust_player: OnReady<Gd<RustPlayer>>,
    zombie_scene: OnReady<Gd<PackedScene>>,
    generated: HashSet<Vector2i>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWorld {
    fn init(base: Base<Node2D>) -> Self {
        // We could also initialize those manually inside ready(), but OnReady automatically defers initialization.
        // Alternatively to init(), you can use #[init(...)] on the struct fields.
        Self {
            tile_map_layer: OnReady::from_node("TileMapLayer"),
            rust_player: OnReady::from_node("RustPlayer"),
            zombie_scene: OnReady::from_loaded("res://scenes/rust_zombie.tscn"),
            generated: HashSet::new(),
            base,
        }
    }

    fn ready(&mut self) {
        self.generate_world(100);

        // let mut timer = self.base().get_node_as::<Timer>("Timer");
        // timer.connect("timeout", &self.base_mut().callable("generate"));
        // timer.set_wait_time(1.0);
        // timer.set_one_shot(false);
        // timer.set_autostart(true);
        // timer.start();

        self.generate_zombie(Vector2::new(250.0, 0.0));
        self.generate_zombie(Vector2::new(300.0, 0.0));
        self.generate_zombie(Vector2::new(350.0, 0.0));
    }
}

#[godot_api]
impl RustWorld {
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
            "Generated world with {} soil, {} sand and {} glass tiles, cost {}ms",
            soil_array.len(),
            sand_array.len(),
            glass_array.len(),
            Instant::now().duration_since(now).as_millis()
        );
    }

    // todo 批量生成僵尸
    pub fn generate_zombie(&mut self, position: Vector2) {
        if let Some(mut zombie) = self.zombie_scene.try_instantiate_as::<RustZombie>() {
            zombie.set_global_position(position);
            self.base_mut().add_child(&zombie);
        }
    }
}
