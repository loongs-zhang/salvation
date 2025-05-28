use crate::MESSAGE;
use crate::common::RustMessage;
use crate::player::RustPlayer;
use crate::zombie::boss::RustBoss;
use godot::builtin::{Array, Vector2, real};
use godot::classes::tween::{EaseType, TransitionType};
use godot::classes::{Area2D, AudioStream, AudioStreamPlayer2D, IArea2D, Node2D};
use godot::global::godot_error;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, OnReady, WithBaseField};
use godot::register::{GodotClass, godot_api};
use godot::tools::load;
use std::sync::LazyLock;

#[allow(clippy::declare_interior_mutable_const)]
const HIT_AUDIOS: LazyLock<Array<Gd<AudioStream>>> = LazyLock::new(|| {
    let mut audios = Array::new();
    for i in 1..=2 {
        audios.push(&load(&format!(
            "res://asserts/player/knifes/katana/katana_hit{}.wav",
            i
        )));
    }
    audios
});

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct RustKnife {
    #[export]
    max_attack_angle: real,
    #[export]
    anticlockwise: bool,
    final_repel: real,
    final_damage: i64,
    damage_area: OnReady<Gd<Area2D>>,
    chop_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    hit_audio: OnReady<Gd<AudioStreamPlayer2D>>,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for RustKnife {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            max_attack_angle: 67.5,
            anticlockwise: false,
            final_repel: 0.0,
            final_damage: 0,
            damage_area: OnReady::from_node("DamageArea"),
            chop_audio: OnReady::from_node("ChopAudio"),
            hit_audio: OnReady::from_node("HitAudio"),
            base,
        }
    }

    fn exit_tree(&mut self) {
        self.hit_audio.set_stream(Gd::null_arg());
    }

    fn ready(&mut self) {
        let gd = self.to_gd();
        self.damage_area
            .signals()
            .body_entered()
            .connect_obj(&gd, Self::on_area_2d_body_entered);
    }
}

#[godot_api]
impl RustKnife {
    pub fn chop(&mut self, final_damage: i64, final_repel: real) {
        if self.base().is_visible() {
            return;
        }
        self.final_damage = final_damage;
        self.final_repel = final_repel;
        let max_attack_angle = if self.anticlockwise {
            -self.max_attack_angle
        } else {
            self.max_attack_angle
        };
        self.base_mut()
            .set_global_rotation_degrees(-max_attack_angle);
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
                &max_attack_angle.to_radians().to_variant(),
                0.2,
            )
            .expect("tween failed")
            .from(&(-max_attack_angle).to_radians().to_variant());
        tween.tween_callback(&self.base().callable("hide"));
        self.chop_audio.play();
    }

    #[func]
    pub fn on_area_2d_body_entered(&mut self, mut body: Gd<Node2D>) {
        if !self.base().is_visible() {
            return;
        }
        let position = self.base().get_global_position();
        let mut damage = 0;
        if body.is_class("RustZombie") || body.is_class("RustBoomer") {
            damage = self.final_damage;
            #[allow(clippy::borrow_interior_mutable_const)]
            if let Some(audio) = HIT_AUDIOS.pick_random() {
                self.hit_audio.set_stream(&audio);
                self.hit_audio.play();
            }
            let zombie_position = body.get_global_position();
            let direction = position.direction_to(zombie_position);
            // 暗杀判定
            if self.try_assassinate(&mut body) {
                damage *= 3;
                #[allow(clippy::borrow_interior_mutable_const)]
                if let Some(mut assassinate_label) = MESSAGE.try_instantiate_as::<RustMessage>() {
                    assassinate_label.set_global_position(position);
                    if let Some(tree) = self.base().get_tree() {
                        if let Some(mut root) = tree.get_root() {
                            root.add_child(&assassinate_label);
                            assassinate_label.bind_mut().show_message("ASSASSINATE");
                        }
                    }
                }
            }
            if !body.is_instance_valid() {
                return;
            }
            body.call_deferred(
                "on_hit",
                &[
                    damage.to_variant(),
                    direction.to_variant(),
                    self.final_repel.to_variant(),
                    (zombie_position + direction).to_variant(),
                ],
            );
        } else if body.is_class("RustBoss") {
            damage = self.final_damage;
            #[allow(clippy::borrow_interior_mutable_const)]
            if let Some(audio) = HIT_AUDIOS.pick_random() {
                self.hit_audio.set_stream(&audio);
                self.hit_audio.play();
            }
            let mut boss = body.cast::<RustBoss>();
            let boss_position = boss.get_global_position();
            let direction = position.direction_to(boss_position);
            boss.bind_mut().on_hit(
                damage,
                direction,
                self.final_repel,
                boss_position + direction,
            );
        } else if body.is_class("RustPlayer") {
            // ok
        } else {
            godot_error!("Knife hit an unexpected body: {}", body.get_class());
        }
        if damage > 0 {
            RustPlayer::get().call_deferred("add_score", &[damage.to_variant()]);
        }
    }

    #[func]
    pub fn hide(&mut self) {
        self.base_mut().set_visible(false);
        RustPlayer::get().bind_mut().chopped();
    }

    // 僵尸背对玩家，则判定可暗杀
    pub fn try_assassinate(&self, zombie: &mut Gd<Node2D>) -> bool {
        let zombie_position = zombie.get_global_position();
        let player_position = RustPlayer::get_position();
        let to_player_dir = player_position.direction_to(zombie_position).normalized();
        let angle = zombie
            .call("get_current_direction", &[])
            .to::<Vector2>()
            .angle_to(to_player_dir)
            .to_degrees();
        (-60.0..=60.0).contains(&angle)
    }
}
