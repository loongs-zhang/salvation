use dashmap::DashSet;
use godot::builtin::{Array, Vector2i, array};
use godot::classes::fast_noise_lite::NoiseType;
use godot::classes::{FastNoiseLite, INode2D, Node2D, TileMapLayer};
use godot::obj::{Base, Gd, NewGd, OnReady};
use godot::register::{GodotClass, godot_api};
use std::sync::LazyLock;

const SOIL_TERRAIN_SET: i32 = 0;
const SAND_TERRAIN_SET: i32 = 1;
const GLASS_TERRAIN_SET: i32 = 2;
const SOURCE_ID: i32 = 0;

static GENERATED: LazyLock<DashSet<Vector2i>> = LazyLock::new(DashSet::new);

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustGround {
    #[export]
    from: Vector2i,
    #[export]
    to: Vector2i,
    tile_map_layer: OnReady<Gd<TileMapLayer>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustGround {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            from: Vector2i::ZERO,
            to: Vector2i::ZERO,
            tile_map_layer: OnReady::from_node("TileMapLayer"),
            base,
        }
    }

    fn ready(&mut self) {
        self.generate(self.from, self.to);
    }
}

#[godot_api]
impl RustGround {
    pub fn generate(&mut self, from: Vector2i, to: Vector2i) {
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
        // generate ground
        for x in from.x..to.x {
            for y in from.y..to.y {
                let vector2i = Vector2i::new(x, y);
                if GENERATED.contains(&vector2i) {
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
                GENERATED.insert(vector2i);
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
    }
}
