use super::*;
use crate::{PLAYER_LEVEL_UP_GROW_RATE, PlayerUpgrade};

#[godot_api(secondary)]
impl RustPlayer {
    pub fn level_up(&mut self) {
        if self.score < self.current_level_up_barrier {
            return;
        }
        //防止重复触发升级
        let damage = self
            .damage
            .saturating_add(self.get_current_weapon().bind().get_damage());
        self.add_score(damage as u64);
        self.level_up_barrier = (self.level_up_barrier as real * PLAYER_LEVEL_UP_GROW_RATE) as u32;
        self.current_level_up_barrier += self.level_up_barrier as u64;
        RustWorld::pause();
        self.hud.bind_mut().set_upgrade_visible(true);
    }

    #[func]
    pub fn upgrade_penetrate(&mut self) {
        //穿透力升级
        self.penetrate += 0.1;
        let new_penetrate = self.penetrate + self.get_current_weapon().bind().get_penetrate();
        self.hud.bind_mut().update_penetrate_hud(new_penetrate);
        self.show_upgrade_label(PlayerUpgrade::Penetrate);
    }

    #[func]
    pub fn upgrade_damage(&mut self) {
        //伤害升级
        self.damage = self.damage.saturating_add(2);
        let new_damage = self
            .damage
            .saturating_add(self.get_current_weapon().bind().get_damage());
        self.hud.bind_mut().update_damage_hud(new_damage);
        self.show_upgrade_label(PlayerUpgrade::Damage);
    }

    #[func]
    pub fn upgrade_repel(&mut self) {
        //击退力升级
        self.repel += 1.0;
        let new_repel = self.repel + self.get_current_weapon().bind().get_repel();
        self.hud.bind_mut().update_repel_hud(new_repel);
        self.show_upgrade_label(PlayerUpgrade::Repel);
    }

    #[func]
    pub fn upgrade_lives(&mut self) {
        //奖励生命数
        self.lives = self.lives.saturating_add(1);
        self.current_lives = self.current_lives.saturating_add(1);
        self.hud
            .bind_mut()
            .update_lives_hud(self.current_lives, self.lives);
        self.show_upgrade_label(PlayerUpgrade::Lives);
    }

    #[func]
    pub fn upgrade_distance(&mut self) {
        //射击距离升级
        self.distance += 20.0;
        let new_distance = self.distance + self.get_current_weapon().bind().get_distance();
        self.hud.bind_mut().update_distance_hud(new_distance);
        self.show_upgrade_label(PlayerUpgrade::Distance);
    }

    #[func]
    pub fn upgrade_health(&mut self) {
        //生命值升级
        self.health = self.health.saturating_add(10);
        self.current_health = self.current_health.saturating_add(10);
        self.hud
            .bind_mut()
            .update_hp_hud(self.current_health, self.health);
        self.show_upgrade_label(PlayerUpgrade::Health);
    }

    fn show_upgrade_label(&mut self, what: PlayerUpgrade) {
        self.hud.bind_mut().set_upgrade_visible(false);
        if let Some(mut level_up_label) = self.create_message() {
            level_up_label.bind_mut().show_level_up(what);
        }
        RustWorld::resume();
    }
}
