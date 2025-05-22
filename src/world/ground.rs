use godot::builtin::{Array, Vector2i, array};
use godot::classes::fast_noise_lite::NoiseType;
use godot::classes::{FastNoiseLite, INode2D, Node2D, TileMapLayer};
use godot::global::godot_error;
use godot::obj::{Base, Gd, NewGd, OnReady};
use godot::register::{GodotClass, godot_api};

const SOIL_TERRAIN_SET: i32 = 0;
const SAND_TERRAIN_SET: i32 = 1;
const GLASS_TERRAIN_SET: i32 = 2;
const SOURCE_ID: i32 = 0;

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustGround {
    #[export]
    points: Array<Vector2i>,
    tile_map_layer: OnReady<Gd<TileMapLayer>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustGround {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            points: Array::new(),
            tile_map_layer: OnReady::from_node("TileMapLayer"),
            base,
        }
    }

    fn draw(&mut self) {
        if self.points.is_empty() {
            godot_error!("Ground points are empty");
            return;
        }
        self.generate();
    }
}

#[godot_api]
impl RustGround {
    pub fn generate(&mut self) {
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
        for point in self.points.iter_shared() {
            let val = noise.get_noise_2d(point.x as f32, point.y as f32);
            if val <= 0.0 {
                soil_array.push(point);
            } else if val <= 0.1 {
                sand_array.push(point);
            } else if val <= 0.2 {
                self.tile_map_layer
                    .set_cell_ex(point)
                    .source_id(SOURCE_ID)
                    .atlas_coords(
                        glass_atlas
                            .pick_random()
                            .expect("Atlas should not be empty"),
                    )
                    .done();
            } else if val <= 0.4 {
                glass_array.push(point);
            } else {
                soil_array.push(point);
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
