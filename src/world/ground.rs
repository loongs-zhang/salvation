use crate::player::RustPlayer;
use crate::world::RustWorld;
use godot::builtin::{PackedVector2Array, Rect2i, Vector2, Vector2i};
use godot::classes::fast_noise_lite::NoiseType;
use godot::classes::{FastNoiseLite, INode2D, Node2D, TileMapLayer};
use godot::obj::{Base, Gd, NewGd, OnReady};
use godot::register::{GodotClass, godot_api};
use std::collections::HashMap;

const GRASS_ATLAS_POSITION: Vector2i = Vector2i::new(0, 2);
const DIRT_ATLAS_POSITION: Vector2i = Vector2i::new(2, 2);
const BUSH_ATLAS_POSITION: Vector2i = Vector2i::new(1, 1);
const MUSHROOM_ATLAS_POSITION: Vector2i = Vector2i::new(3, 0);
const ROCK_ATLAS_POSITION: Vector2i = Vector2i::new(2, 1);
#[allow(dead_code)]
const WATER_ATLAS_POSITION: Vector2i = Vector2i::new(1, 2);

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct RustGround {
    #[export]
    tile_size: i32,
    #[export]
    chunk_size_x: i32,
    #[export]
    chunk_size_y: i32,
    view_distance_x: i32,
    view_distance_y: i32,
    noise: Gd<FastNoiseLite>,
    object_placed_range: Rect2i,
    object_tiles_position: HashMap<i32, PackedVector2Array>,
    other_tiles_position: PackedVector2Array,
    objects_high: OnReady<Gd<TileMapLayer>>,
    objects: OnReady<Gd<TileMapLayer>>,
    ground: OnReady<Gd<TileMapLayer>>,
    other: OnReady<Gd<TileMapLayer>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for RustGround {
    fn init(base: Base<Node2D>) -> Self {
        let mut object_tiles_position = HashMap::new();
        object_tiles_position.insert(0, PackedVector2Array::new());
        object_tiles_position.insert(1, PackedVector2Array::new());
        object_tiles_position.insert(2, PackedVector2Array::new());
        object_tiles_position.insert(3, PackedVector2Array::new());
        Self {
            tile_size: 32,
            chunk_size_x: 64,
            chunk_size_y: 42,
            view_distance_x: 16,
            view_distance_y: 16,
            noise: FastNoiseLite::new_gd(),
            object_placed_range: Rect2i::new(Vector2i::ZERO, Vector2i::ZERO),
            object_tiles_position,
            other_tiles_position: PackedVector2Array::new(),
            objects_high: OnReady::from_node("ObjectsHigh"),
            objects: OnReady::from_node("Objects"),
            ground: OnReady::from_node("Ground"),
            other: OnReady::from_node("Other"),
            base,
        }
    }

    fn physics_process(&mut self, _delta: f64) {
        // Chunk Loading/Unloading
        let world_size = self.ground.get_used_rect();
        let world_size_start = world_size.position;
        let world_size_end = world_size_start + world_size.size;
        let player_position = RustPlayer::get_position().cast_int();
        let player_position_positive = (player_position / self.tile_size)
            + Vector2i::new(self.view_distance_x, self.view_distance_y);
        let player_position_negative = (player_position / self.tile_size)
            - Vector2i::new(self.view_distance_x, self.view_distance_y);

        if player_position_positive.x > world_size_end.x {
            self.load_chunk(world_size_start.x + 1, world_size_start.y);
        } else if player_position_negative.x < world_size_start.x {
            self.load_chunk(world_size_start.x - 1, world_size_start.y);
        }
        if player_position_positive.y > world_size_end.y {
            self.load_chunk(world_size_start.x, world_size_start.y + 1);
        } else if player_position_negative.y < world_size_start.y {
            self.load_chunk(world_size_start.x, world_size_start.y - 1);
        }
    }

    fn ready(&mut self) {
        self.noise.set_noise_type(NoiseType::PERLIN);
        self.noise.set_seed(rand::random::<i32>());
        self.noise.set_frequency(0.03);
        self.noise.set_domain_warp_amplitude(1.0);
        self.view_distance_x = (self.chunk_size_x / 2) - 1;
        self.view_distance_y = (self.chunk_size_y / 2) - 1;
        let player_position = RustPlayer::get_position().cast_int();
        self.load_chunk(player_position.x, player_position.y);
    }
}

#[godot_api]
impl RustGround {
    pub fn load_chunk(&mut self, x: i32, y: i32) {
        self.objects_high.clear();
        self.objects.clear();
        self.ground.clear();
        self.other.clear();

        for _x in 0..self.chunk_size_x {
            for _y in 0..self.chunk_size_y {
                let current_tile_position = Vector2i::new(_x + x, _y + y);
                #[allow(unused_assignments)]
                let mut atlas_position = Vector2i::ZERO;
                let id = self.noise.get_noise_2d((_x + x) as f32, (_y + y) as f32);
                // no need for water
                // if id > 0.4 {
                //     atlas_position = WATER_ATLAS_POSITION;
                // }
                if id > 0.2 {
                    atlas_position = GRASS_ATLAS_POSITION;
                } else {
                    atlas_position = DIRT_ATLAS_POSITION;
                }

                if self.ground.get_cell_source_id(current_tile_position) == -1 {
                    self.ground
                        .set_cell_ex(current_tile_position)
                        .source_id(0)
                        .atlas_coords(atlas_position)
                        .done();
                }
                // Object Placement
                if rand::random::<i32>() % 25 == 0
                    && !self
                        .object_placed_range
                        .contains_point(current_tile_position)
                {
                    for (i, array) in &mut self.object_tiles_position {
                        match i {
                            0 => {
                                if rand::random::<i32>() % 2 == 0 && atlas_position.x == 0 {
                                    array.push(current_tile_position.cast_float());
                                }
                            }
                            1 => {
                                if rand::random::<i32>() % 2 == 1 && atlas_position.x == 0 {
                                    array.push(current_tile_position.cast_float());
                                }
                            }
                            2 => {
                                if atlas_position.x == 2 {
                                    array.push(current_tile_position.cast_float());
                                }
                            }
                            3 => {
                                if rand::random::<i32>() % 2 == 0 && atlas_position.x == 0 {
                                    array.push(current_tile_position.cast_float());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        let ground_rect = self.ground.get_used_rect();
        if !self
            .object_placed_range
            .contains_point(ground_rect.position)
            || !self
                .object_placed_range
                .contains_point(ground_rect.position + ground_rect.size)
        {
            self.object_placed_range = self.object_placed_range.merge(ground_rect)
        }
        for (t, array) in self.object_tiles_position.clone() {
            match t {
                0 => {
                    for tp in 0..array.len() {
                        if self
                            .ground
                            .get_cell_source_id(Vector2::new(array[tp].x, array[tp].y).cast_int())
                            != -1
                            || self.ground.get_cell_source_id(
                                Vector2::new(array[tp].x, array[tp].y + 1.0).cast_int(),
                            ) == -1
                        {
                            self.draw_tree(array[tp].x as i32, array[tp].y as i32);
                        }
                    }
                }
                1 => {
                    for tp in 0..array.len() {
                        if self
                            .ground
                            .get_cell_source_id(Vector2::new(array[tp].x, array[tp].y).cast_int())
                            != -1
                        {
                            self.draw_bush(array[tp].x as i32, array[tp].y as i32)
                        }
                    }
                }
                2 => {
                    for tp in 0..array.len() {
                        if self
                            .ground
                            .get_cell_source_id(Vector2::new(array[tp].x, array[tp].y).cast_int())
                            != -1
                        {
                            self.draw_rock(array[tp].x as i32, array[tp].y as i32)
                        }
                    }
                }
                3 => {
                    for tp in 0..array.len() {
                        if self
                            .ground
                            .get_cell_source_id(Vector2::new(array[tp].x, array[tp].y).cast_int())
                            != -1
                        {
                            self.draw_mushroom(array[tp].x as i32, array[tp].y as i32)
                        }
                    }
                }
                _ => {}
            }
        }
        for op in 0..self.other_tiles_position.len() {
            if self
                .ground
                .get_cell_source_id(self.other_tiles_position[op].cast_int())
                != -1
            {
                self.other
                    .set_cell_ex(self.other_tiles_position[op].cast_int())
                    .source_id(0)
                    .atlas_coords(Vector2i::new(3, 2))
                    .done();
            }
        }
    }

    fn draw_tree(&mut self, x: i32, y: i32) {
        if self.ground.get_cell_source_id(Vector2i::new(x, y - 1)) != -1 {
            self.objects_high
                .set_cell_ex(Vector2i::new(x, y - 1))
                .source_id(0)
                .atlas_coords(Vector2i::new(0, 0))
                .done();
        }
        if self.ground.get_cell_source_id(Vector2i::new(x, y)) != -1 {
            self.objects_high
                .set_cell_ex(Vector2i::new(x, y))
                .source_id(0)
                .atlas_coords(Vector2i::new(0, 1))
                .done();
        }
        if self
            .objects_high
            .get_cell_atlas_coords(Vector2i::new(x, y - 2))
            == Vector2i::new(0, 0)
        {
            self.objects_high.erase_cell(Vector2i::new(x, y - 2))
        }
    }

    fn draw_bush(&mut self, x: i32, y: i32) {
        if self.ground.get_cell_source_id(Vector2i::new(x, y)) != -1
            && self.objects_high.get_cell_source_id(Vector2i::new(x, y)) == -1
        {
            self.objects_high
                .set_cell_ex(Vector2i::new(x, y))
                .source_id(0)
                .atlas_coords(BUSH_ATLAS_POSITION)
                .done();
        }
    }

    fn draw_rock(&mut self, x: i32, y: i32) {
        if self.ground.get_cell_source_id(Vector2i::new(x, y)) != -1
            && self.objects.get_cell_source_id(Vector2i::new(x, y)) == -1
        {
            self.objects
                .set_cell_ex(Vector2i::new(x, y))
                .source_id(0)
                .atlas_coords(ROCK_ATLAS_POSITION)
                .done()
        }
    }

    fn draw_mushroom(&mut self, x: i32, y: i32) {
        if self.ground.get_cell_source_id(Vector2i::new(x, y)) != -1
            && self.objects.get_cell_source_id(Vector2i::new(x, y)) == -1
        {
            self.objects
                .set_cell_ex(Vector2i::new(x, y))
                .source_id(0)
                .atlas_coords(MUSHROOM_ATLAS_POSITION)
                .done()
        }
    }

    pub fn get() -> Gd<Self> {
        RustWorld::get().get_node_as::<Self>("RustGround")
    }

    pub fn get_objects_z_index() -> i32 {
        Self::get().bind().objects.get_z_index()
    }
}
