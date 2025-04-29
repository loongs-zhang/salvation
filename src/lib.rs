use godot::classes::fast_noise_lite::NoiseType;
use godot::classes::{FastNoiseLite, TileMapLayer};
use godot::prelude::*;

struct Salvation;

#[gdextension]
unsafe impl ExtensionLibrary for Salvation {}

const WIDTH: i32 = 640;
const HEIGHT: i32 = 360;

const SOIL_TERRAIN_SET: i32 = 0;
const SAND_TERRAIN_SET: i32 = 1;
const GLASS_TERRAIN_SET: i32 = 2;
const SOURCE_ID: i32 = 0;

// Deriving GodotClass makes the class available to Godot.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWorld {
    play_scene: OnReady<Gd<PackedScene>>,
    tile_map_layer: OnReady<Gd<TileMapLayer>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustWorld {
    fn init(base: Base<Node2D>) -> Self {
        // We could also initialize those manually inside ready(), but OnReady automatically defers initialization.
        // Alternatively to init(), you can use #[init(...)] on the struct fields.
        Self {
            // OnReady::from_loaded(path) == OnReady::new(|| tools::load(path)).
            play_scene: OnReady::from_loaded("res://scenes/rust_world.tscn"),
            tile_map_layer: OnReady::from_node("TileMapLayer"),
            base,
        }
    }

    fn ready(&mut self) {
        let width = WIDTH / 2;
        let height = HEIGHT / 2;
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
        for x in -width..width {
            for y in -height..height {
                let val = noise.get_noise_2d(x as f32, y as f32);
                if val <= 0.0 {
                    soil_array.push(Vector2i::new(x, y));
                } else if 0.0 >= val && val <= 0.1 {
                    sand_array.push(Vector2i::new(x, y));
                } else if 0.1 >= val && val <= 0.2 {
                    self.tile_map_layer
                        .set_cell_ex(Vector2i::new(x, y))
                        .source_id(SOURCE_ID)
                        .atlas_coords(
                            glass_atlas
                                .pick_random()
                                .expect("Atlas should not be empty"),
                        )
                        .done();
                } else {
                    glass_array.push(Vector2i::new(x, y));
                }
            }
        }
        self.tile_map_layer
            .set_cells_terrain_connect(&soil_array, SOIL_TERRAIN_SET, 0);
        self.tile_map_layer
            .set_cells_terrain_connect(&sand_array, SAND_TERRAIN_SET, 0);
        self.tile_map_layer
            .set_cells_terrain_connect(&glass_array, GLASS_TERRAIN_SET, 0);
    }
}
