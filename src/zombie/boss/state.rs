use super::*;
use crate::world::ground::RustGround;

#[godot_api(secondary)]
impl RustBoss {
    #[func]
    pub fn guard(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.75;
        self.state = ZombieState::Guard;
        if !self.guard_audio.is_playing() && self.guard_audio.is_inside_tree() {
            self.guard_audio.play();
        }
        self.notify_animation();
    }

    pub fn bump(&mut self) {
        if ZombieState::Run == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("bump").done();
        self.current_speed = self.speed * 1.5;
        self.state = ZombieState::Run;
        if !self.bump_audio.is_playing() && self.bump_audio.is_inside_tree() {
            self.bump_audio.play();
        }
        self.notify_animation();
    }

    pub fn hit(&mut self, direction: Vector2, hit_position: Vector2) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.2;
        self.state = ZombieState::Hit;
        self.blood_flash.set_global_position(hit_position);
        self.blood_flash.look_at(hit_position - direction);
        self.blood_flash.restart();
        if self.hit_audio.is_inside_tree() {
            self.hit_audio.play();
        }
        if self.scream_audio.is_inside_tree() {
            self.scream_audio.play();
        }
        self.notify_animation();
    }

    pub fn attack(&mut self) {
        if ZombieState::Dead == self.state || !self.attackable {
            return;
        }
        self.base_mut().look_at(RustPlayer::get_position());
        self.animated_sprite2d.play_ex().name("attack").done();
        self.current_speed = self.speed * 0.75;
        self.state = ZombieState::Attack;
        let direction = NEXT_ATTACK_DIRECTION.load()
            + self
                .base()
                .get_global_position()
                .direction_to(RustPlayer::get_position());
        NEXT_ATTACK_DIRECTION.store(direction.normalized());
        if self.attack_audio.is_inside_tree() {
            self.attack_audio.play();
        }
        if !self.attack_scream_audio.is_playing() && self.attack_scream_audio.is_inside_tree() {
            self.attack_scream_audio.play();
        }
        self.notify_animation();
    }

    #[func]
    pub fn die(&mut self) {
        if ZombieState::Dead == self.state {
            return;
        }
        self.animated_sprite2d.play_ex().name("die").done();
        self.current_speed = 0.0;
        self.state = ZombieState::Dead;
        if self.die_audio.is_inside_tree() {
            self.die_audio.play();
        }
        // 释放资源
        self.hud.queue_free();
        self.head_shape2d.queue_free();
        self.collision_shape2d.queue_free();
        self.bump_damage_area.queue_free();
        self.zombie_attack_area.queue_free();
        self.zombie_damage_area.queue_free();
        self.born_audio.queue_free();
        self.hit_audio.queue_free();
        self.blood_flash.queue_free();
        self.scream_audio.queue_free();
        self.guard_audio.queue_free();
        self.bump_audio.queue_free();
        self.attack_audio.queue_free();
        self.attack_scream_audio.queue_free();
        self.notify_animation();
        // 45S后自动清理尸体
        BODY_COUNT.fetch_add(1, Ordering::Release);
        self.base_mut()
            .set_z_index(RustGround::get_objects_z_index());
        if let Some(mut tree) = self.base().get_tree() {
            if let Some(mut timer) = tree.create_timer(45.0) {
                timer.connect("timeout", &self.base().callable("clean_body"));
            }
        }
        // 击杀僵尸确认
        if let Some(mut level) = RustLevel::get() {
            level.bind_mut().kill_boss_confirmed();
        }
    }
}
