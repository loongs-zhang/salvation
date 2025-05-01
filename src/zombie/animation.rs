use crate::player::RustPlayer;
use crate::zombie::{DAMAGE, ZombieState};
use godot::builtin::real;
use godot::classes::{AnimatedSprite2D, IAnimatedSprite2D, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

const HURT_FRAME: [i32; 4] = [2, 3, 4, 5];

#[derive(GodotClass)]
#[class(base=AnimatedSprite2D)]
pub struct ZombieAnimation {
    state: ZombieState,
    base: Base<AnimatedSprite2D>,
}

#[godot_api]
impl IAnimatedSprite2D for ZombieAnimation {
    fn init(base: Base<AnimatedSprite2D>) -> Self {
        Self {
            state: ZombieState::Guard,
            base,
        }
    }

    fn ready(&mut self) {
        self.signals()
            .frame_changed()
            .connect_self(Self::on_animated_sprite_2d_frame_changed);
        if let Some(tree) = self.base().get_parent() {
            if let Some(tree) = tree.get_parent() {
                tree.get_node_as::<RustPlayer>("RustPlayer")
                    .signals()
                    .hit()
                    .connect_self(RustPlayer::on_hit);
            }
        }
    }
}

#[godot_api]
impl ZombieAnimation {
    #[signal]
    pub fn change_state(state: ZombieState);

    #[func]
    pub fn on_change_state(&mut self, state: ZombieState) {
        self.state = state;
    }

    #[signal]
    pub fn change_attack_animation(repeat: bool);

    #[func]
    pub fn on_change_attack_animation(&mut self, repeat: bool) {
        let mut base = self.base_mut();
        if let Some(mut frames) = base.get_sprite_frames() {
            frames.set_animation_loop("attack", repeat);
            if repeat {
                base.play_ex().name("attack").done();
            }
        }
    }

    #[func]
    pub fn on_animated_sprite_2d_frame_changed(&mut self) {
        let base = self.base();
        // todo 改为体积碰撞检测
        let distance = self.get_distance();
        if !RustPlayer::is_dead()
            && ZombieState::Attack == self.state
            && distance <= 120.0
            && base.get_animation() == "attack".into()
            && HURT_FRAME.contains(&base.get_frame())
        {
            if let Some(tree) = base.get_parent() {
                if let Some(tree) = tree.get_parent() {
                    // 伤害玩家
                    tree.get_node_as::<RustPlayer>("RustPlayer")
                        .signals()
                        .hit()
                        .emit(DAMAGE);
                }
            }
        }
    }

    pub fn get_distance(&self) -> real {
        let zombie_position = self.base().get_global_position();
        let player_position = RustPlayer::get_position();
        zombie_position.distance_to(player_position)
    }
}
