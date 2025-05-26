use crate::player::RustPlayer;
use crate::{BOSS_DAMAGE, ZombieState};
use godot::classes::{Area2D, IArea2D, Node, Node2D, Object};
use godot::obj::{Base, Gd, WithBaseField, WithUserSignals};
use godot::register::{GodotClass, godot_api};
use std::time::{Duration, Instant};

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct BossBumpArea {
    boss_state: ZombieState,
    last_bump_time: Instant,
    bump_cooldown: Duration,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for BossBumpArea {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            boss_state: ZombieState::Guard,
            last_bump_time: Instant::now(),
            bump_cooldown: Duration::from_secs(10),
            base,
        }
    }

    fn ready(&mut self) {
        self.last_bump_time -= self.bump_cooldown;
        self.signals()
            .change_zombie_state()
            .connect_self(Self::on_change_zombie_state);
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
        let now = Instant::now();
        if (ZombieState::Run == self.boss_state || ZombieState::Attack == self.boss_state)
            && body.is_class("RustPlayer")
            && now.duration_since(self.last_bump_time) >= self.bump_cooldown
        {
            // 撞击玩家，如果无冷却就会一直撞击，不攻击
            let position = self.base().get_global_position();
            self.get_rust_player()
                .bind_mut()
                .on_impact(BOSS_DAMAGE * 4, position);
            self.last_bump_time = now;
        }
    }

    pub fn get_rust_player(&mut self) -> Gd<RustPlayer> {
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
