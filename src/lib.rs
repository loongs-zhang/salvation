use godot::init::{ExtensionLibrary, gdextension};

pub mod world;

pub mod player;

struct Salvation;

#[gdextension]
unsafe impl ExtensionLibrary for Salvation {}
