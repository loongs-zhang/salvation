use godot::init::{ExtensionLibrary, gdextension};

pub mod world;

pub mod player;

pub mod weapon;

pub mod bullet;

pub mod zombie;

struct Salvation;

#[gdextension]
unsafe impl ExtensionLibrary for Salvation {}
