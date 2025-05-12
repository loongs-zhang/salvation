use crate::player::RustPlayer;
use crate::{PlayerState, ZombieState};
use godot::classes::{AnimatedSprite2D, IAnimatedSprite2D, Node, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=AnimatedSprite2D)]
pub struct ZombieAnimation {
    damage: i64,
    hurt_frames: Vec<i32>,
    player_in_area: bool,
    zombie_state: ZombieState,
    base: Base<AnimatedSprite2D>,
}

#[godot_api]
impl IAnimatedSprite2D for ZombieAnimation {
    fn init(base: Base<AnimatedSprite2D>) -> Self {
        Self {
            damage: 0,
            hurt_frames: Vec::new(),
            player_in_area: false,
            zombie_state: ZombieState::Guard,
            base,
        }
    }

    fn ready(&mut self) {
        self.signals()
            .frame_changed()
            .connect_self(Self::on_animated_sprite_2d_frame_changed);
    }

    fn process(&mut self, _delta: f64) {
        if ZombieState::Attack != self.zombie_state {
            return;
        }
        let mut base = self.base_mut();
        if let Some(mut frames) = base.get_sprite_frames() {
            match RustPlayer::get_state() {
                PlayerState::Dead => frames.set_animation_loop("attack", false),
                _ => {
                    frames.set_animation_loop("attack", true);
                    base.play_ex().name("attack").done();
                }
            }
        }
    }
}

#[godot_api]
impl ZombieAnimation {
    #[signal]
    pub fn change_zombie_state(zombie_state: ZombieState);

    #[func]
    pub fn on_change_zombie_state(&mut self, zombie_state: ZombieState) {
        self.zombie_state = zombie_state;
    }

    #[signal]
    pub fn player_in_area(player_in_area: bool);

    #[func]
    pub fn on_player_in_area(&mut self, player_in_area: bool) {
        self.player_in_area = player_in_area;
    }

    pub fn set_hurt_frames(&mut self, frames: Vec<i32>) {
        self.hurt_frames = frames;
    }

    pub fn set_damage(&mut self, damage: i64) {
        self.damage = damage;
    }

    #[func]
    pub fn on_animated_sprite_2d_frame_changed(&mut self) {
        let base = self.base();
        if self.player_in_area
            && PlayerState::Dead != RustPlayer::get_state()
            && ZombieState::Attack == self.zombie_state
            && base.get_animation() == "attack".into()
            && self.hurt_frames.contains(&base.get_frame())
        {
            // 伤害玩家
            let position = self.base().get_global_position();
            self.get_rust_player()
                .bind_mut()
                .on_hit(self.damage, position);
        }
    }

    fn get_rust_player(&mut self) -> Gd<RustPlayer> {
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                return root
                    .get_node_as::<Node>("RustWorld")
                    .get_node_as::<RustPlayer>("RustPlayer");
            }
        }
        panic!("RustPlayer not found");
    }
}
