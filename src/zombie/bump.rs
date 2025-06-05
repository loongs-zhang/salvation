use crate::player::RustPlayer;
use crate::zombie::boss::RustBoss;
use crate::{BOSS_DAMAGE, ZombieState, is_survivor};
use godot::classes::{Area2D, IArea2D, Node2D, Object};
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
        if let Ok(boss) = self
            .base()
            .get_parent()
            .expect("ZombieAttackArea parent not found")
            .try_cast::<RustBoss>()
        {
            if !boss.bind().get_collidable() {
                return;
            }
        }
        let now = Instant::now();
        if ZombieState::Dead != self.boss_state
            && is_survivor(&***body)
            && now.duration_since(self.last_bump_time) >= self.bump_cooldown
        {
            // 撞击玩家，如果无冷却就会一直撞击，不攻击
            let position = self.base().get_global_position();
            RustPlayer::get()
                .bind_mut()
                .on_impact(BOSS_DAMAGE * 4, position);
            self.last_bump_time = now;
        }
    }
}
