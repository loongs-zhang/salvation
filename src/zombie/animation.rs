use crate::player::RustPlayer;
use crate::{PlayerState, ZOMBIE_DAMAGE, ZombieState};
use godot::classes::{AnimatedSprite2D, IAnimatedSprite2D, Object};
use godot::global::godot_print;
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

const HURT_FRAME: [i32; 4] = [2, 3, 4, 5];

#[derive(GodotClass)]
#[class(base=AnimatedSprite2D)]
pub struct ZombieAnimation {
    player_in_area: bool,
    player_state: PlayerState,
    zombie_state: ZombieState,
    base: Base<AnimatedSprite2D>,
}

#[godot_api]
impl IAnimatedSprite2D for ZombieAnimation {
    fn init(base: Base<AnimatedSprite2D>) -> Self {
        Self {
            player_in_area: false,
            player_state: PlayerState::Born,
            zombie_state: ZombieState::Guard,
            base,
        }
    }

    fn ready(&mut self) {
        self.signals()
            .frame_changed()
            .connect_self(Self::on_animated_sprite_2d_frame_changed);
        self.get_rust_player()
            .signals()
            .hit()
            .connect_self(RustPlayer::on_hit);
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
    pub fn change_player_state(player_state: PlayerState);

    #[func]
    pub fn on_change_player_state(&mut self, player_state: PlayerState) {
        self.player_state = player_state;
        let mut base = self.base_mut();
        if let Some(mut frames) = base.get_sprite_frames() {
            match player_state {
                PlayerState::Born => {
                    frames.set_animation_loop("attack", true);
                    base.play_ex().name("attack").done();
                }
                PlayerState::Dead => frames.set_animation_loop("attack", false),
                _ => {}
            }
        }
    }

    #[signal]
    pub fn player_in_area(player_in_area: bool);

    #[func]
    pub fn on_player_in_area(&mut self, player_in_area: bool) {
        self.player_in_area = player_in_area;
    }

    #[func]
    pub fn on_animated_sprite_2d_frame_changed(&mut self) {
        let base = self.base();
        if self.player_in_area
            && PlayerState::Dead != self.player_state
            && ZombieState::Attack == self.zombie_state
            && base.get_animation() == "attack".into()
            && HURT_FRAME.contains(&base.get_frame())
        {
            // 伤害玩家
            godot_print!("zombie attack player in frame:{}", base.get_frame());
            self.get_rust_player().signals().hit().emit(ZOMBIE_DAMAGE);
        }
    }

    fn get_rust_player(&mut self) -> Gd<RustPlayer> {
        if let Some(tree) = self.base().get_parent() {
            if let Some(tree) = tree.get_parent() {
                return tree.get_node_as::<RustPlayer>("RustPlayer");
            }
        }
        panic!("RustPlayer not found");
    }
}
