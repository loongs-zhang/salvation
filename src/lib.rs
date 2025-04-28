use godot::classes::fast_noise_lite::NoiseType;
use godot::classes::{FastNoiseLite, Label, TileMapLayer};
use godot::prelude::*;

struct Salvation;

#[gdextension]
unsafe impl ExtensionLibrary for Salvation {
    fn on_level_init(level: InitLevel) {
        println!("init {level:?}");
    }

    fn on_level_deinit(level: InitLevel) {
        println!("deinit {level:?}");
    }
}

const WIDTH: i32 = 960;
const HEIGHT: i32 = 600;
const LAND: Vector2i = Vector2i::new(0, 0);
const WATER: Vector2i = Vector2i::new(0, 1);
const SOURCE_ID: i32 = 0;

// Deriving GodotClass makes the class available to Godot.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustWorld {
    play_scene: OnReady<Gd<PackedScene>>,
    tile_map_layer: OnReady<Gd<TileMapLayer>>,
    message_label: OnReady<Gd<Label>>,
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
            message_label: OnReady::from_node("MessageLabel"),
            base,
        }
    }

    fn ready(&mut self) {
        self.message_label.set_text("try to generate world");
        self.message_label.show();
        // try to generate world
        let mut noise = FastNoiseLite::new_gd();
        noise.set_noise_type(NoiseType::SIMPLEX_SMOOTH);
        noise.set_seed(rand::random::<i32>());
        noise.set_frequency(0.05);
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                let val = noise.get_noise_2d(x as f32, y as f32);
                self.tile_map_layer
                    .set_cell_ex(Vector2i::new(x, y))
                    .source_id(SOURCE_ID)
                    .atlas_coords(if val > 0.1 { LAND } else { WATER })
                    .done();
            }
        }

        self.message_label.set_text("generate world finished");
        self.message_label.show();
    }
}
