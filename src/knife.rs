use crate::boss::RustBoss;
use crate::player::RustPlayer;
use crate::zombie::RustZombie;
use godot::builtin::{Array, real};
use godot::classes::tween::{EaseType, TransitionType};
use godot::classes::{Area2D, AudioStream, AudioStreamPlayer2D, IArea2D, Node2D};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::prelude::load;
use godot::register::{GodotClass, godot_api};

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct RustKnife {
    #[export]
    chop_cooldown: real,
    final_repel: real,
    final_damage: i64,
    current_chop_cooldown: f64,
    damage_area: OnReady<Gd<Area2D>>,
    chop_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hit_audios: Array<Gd<AudioStream>>,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for RustKnife {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            chop_cooldown: 0.5,
            final_repel: 0.0,
            final_damage: 0,
            current_chop_cooldown: 0.0,
            damage_area: OnReady::from_node("DamageArea"),
            chop_audio: OnReady::from_node("ChopAudio"),
            hit_audio: OnReady::from_node("HitAudio"),
            hit_audios: Array::new(),
            base,
        }
    }

    fn ready(&mut self) {
        if self.hit_audios.is_empty() {
            for i in 1..=2 {
                self.hit_audios.push(&load(&format!(
                    "res://asserts/player/knifes/katana/katana_hit{}.wav",
                    i
                )));
            }
        }
        let gd = self.to_gd();
        self.damage_area
            .signals()
            .body_entered()
            .connect_obj(&gd, Self::on_area_2d_body_entered);
    }

    fn process(&mut self, delta: f64) {
        self.current_chop_cooldown -= delta;
    }
}

#[godot_api]
impl RustKnife {
    pub fn chop(&mut self, final_damage: i64, final_repel: real) {
        if self.current_chop_cooldown > 0.0 {
            return;
        }
        self.current_chop_cooldown = self.chop_cooldown as f64;
        self.final_damage = final_damage;
        self.final_repel = final_repel;
        self.base_mut().set_global_rotation_degrees(-60.0);
        self.base_mut().set_visible(true);
        let mut tween = self
            .base_mut()
            .create_tween()
            .expect("Failed to create tween");
        tween.set_ease(EaseType::IN_OUT);
        tween.set_trans(TransitionType::QUAD);
        tween
            .tween_property(
                &self.base.to_gd(),
                "rotation",
                &60.0f32.to_radians().to_variant(),
                0.2,
            )
            .expect("tween failed")
            .from(&(-60.0f32).to_radians().to_variant());
        tween.tween_callback(&self.base().callable("hide"));
        self.chop_audio.play();
    }

    #[func]
    pub fn on_area_2d_body_entered(&mut self, body: Gd<Node2D>) {
        let position = self.base().get_global_position();
        if body.is_class("RustZombie") {
            if let Some(audio) = self.hit_audios.pick_random() {
                self.hit_audio.set_stream(&audio);
                self.hit_audio.play();
            }
            let mut zombie = body.cast::<RustZombie>();
            let zombie_position = zombie.get_global_position();
            let direction = position.direction_to(zombie_position);
            zombie.bind_mut().on_hit(
                self.final_damage,
                direction,
                self.final_repel,
                zombie_position + direction,
            );
            if self.final_damage > 0 {
                RustPlayer::add_score(self.final_damage as u64);
            }
        } else if body.is_class("RustBoss") {
            if let Some(audio) = self.hit_audios.pick_random() {
                self.hit_audio.set_stream(&audio);
                self.hit_audio.play();
            }
            let mut boss = body.cast::<RustBoss>();
            let boss_position = boss.get_global_position();
            let direction = position.direction_to(boss_position);
            boss.bind_mut().on_hit(
                self.final_damage,
                direction,
                self.final_repel,
                boss_position + direction,
            );
            if self.final_damage > 0 {
                RustPlayer::add_score(self.final_damage as u64);
            }
        }
    }

    #[func]
    pub fn hide(&mut self) {
        self.base_mut().set_visible(false);
        if let Some(parent) = self.base().get_parent() {
            if parent.is_class("RustPlayer") {
                parent.cast::<RustPlayer>().call_deferred("chopped", &[]);
            }
        };
    }
}
