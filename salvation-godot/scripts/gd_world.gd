extends Node2D

@export var noise_texture: NoiseTexture2D
var noise: Noise
@onready var tile_map = $TileMapLayer;

const width = 960;
const height = 600;
const source_id = 0;
const land_atlas = Vector2i(0, 0);
const water_atlas = Vector2i(0, 1);

func _ready() -> void:
	noise = noise_texture.noise;
	for x in range(width): 
		for y in range(height): 
			var val = noise.get_noise_2d(x, y);
			if val >= 0.0: 
				tile_map.set_cell(Vector2i(x, y), source_id, land_atlas)
			else: 
				tile_map.set_cell(Vector2i(x, y), source_id, water_atlas)
	pass
