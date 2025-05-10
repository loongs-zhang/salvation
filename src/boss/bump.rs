use crate::player::RustPlayer;
use crate::world::RustWorld;
use crate::{BOSS_DAMAGE, ZombieState};
use godot::classes::{Area2D, IArea2D, Node2D, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct BossBumpArea {
    boss_state: ZombieState,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for BossBumpArea {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            boss_state: ZombieState::Guard,
            base,
        }
    }

    fn ready(&mut self) {
        self.signals()
            .body_entered()
            .connect_self(Self::on_area_2d_body_entered);
    }
}

#[godot_api]
impl BossBumpArea {
    #[signal]
    pub fn change_zombie_state(zombie_state: ZombieState);

    #[func]
    pub fn on_change_zombie_state(&mut self, zombie_state: ZombieState) {
        self.boss_state = zombie_state;
    }

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        if (ZombieState::Run == self.boss_state || ZombieState::Attack == self.boss_state)
            && body.is_class("RustPlayer")
        {
            // 伤害玩家
            let position = self.base().get_global_position();
            self.get_rust_player()
                .bind_mut()
                .on_hit(BOSS_DAMAGE, position);
        }
    }

    fn get_rust_player(&mut self) -> Gd<RustPlayer> {
        if let Some(tree) = self.base().get_tree() {
            if let Some(root) = tree.get_root() {
                return root
                    .get_node_as::<RustWorld>("RustWorld")
                    .get_node_as::<RustPlayer>("RustPlayer");
            }
        }
        panic!("RustPlayer not found");
    }
}
