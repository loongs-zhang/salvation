use godot::init::{ExtensionLibrary, gdextension};

pub mod world;

pub mod player;

pub mod bullet;

struct Salvation;

#[gdextension]
unsafe impl ExtensionLibrary for Salvation {}
