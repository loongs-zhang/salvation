use super::*;
use crate::weapon::RustWeapon;
use godot::classes::{DirAccess, Texture2D};
use godot::global::godot_warn;
use std::collections::HashMap;

#[allow(clippy::declare_interior_mutable_const)]
const WEAPON_TEXTURE: LazyLock<HashMap<GString, Gd<Texture2D>>> = LazyLock::new(|| {
    const WEAPONS_DIR: &str = "res://asserts/player/weapons";
    const SUFFIX: &str = "_m.png";
    let mut map = HashMap::new();
    if let Some(mut weapons_dir) = DirAccess::open(WEAPONS_DIR) {
        for dir_name in weapons_dir.get_directories().to_vec() {
            if let Some(mut weapons_dir) = DirAccess::open(&format!("{}/{}", WEAPONS_DIR, dir_name))
            {
                for file in weapons_dir.get_files().to_vec() {
                    if file.ends_with(SUFFIX) {
                        map.insert(
                            file.replace(SUFFIX, "").to_upper(),
                            load(&format!("{}/{}/{}", WEAPONS_DIR, dir_name, file)),
                        );
                    }
                }
            }
        }
    }
    map
});

#[godot_api(secondary)]
impl RustPlayer {
    pub fn change_weapon(&mut self, weapon_index: i32) {
        if PlayerState::Dead == self.state {
            return;
        }
        let weapon_count = self.weapons.get_child_count();
        if weapon_index >= weapon_count {
            if let Some(mut locked_label) = self.create_message() {
                locked_label
                    .bind_mut()
                    .show_message(&format!("WEAPON {} LOCKED", weapon_index + 1));
            }
            return;
        }
        for i in 0..weapon_count {
            if let Some(node) = self.weapons.get_child(i) {
                let mut weapon = node.cast::<RustWeapon>();
                if weapon_index == i {
                    weapon.set_visible(true);
                    self.ray_cast2d
                        .set_target_position(Vector2::new(weapon.bind().get_distance(), 0.0));
                    let mut hud = self.hud.bind_mut();
                    let weapon_name = weapon.get_name().to_upper();
                    hud.update_weapon_name_hud(&if weapon.bind().get_silenced() {
                        format!("SILENCED {}", weapon_name)
                    } else {
                        weapon_name.to_string()
                    });
                    #[allow(clippy::borrow_interior_mutable_const)]
                    if let Some(weapon_texture) = WEAPON_TEXTURE.get(&weapon_name) {
                        hud.update_weapon_sprite_hud(weapon_texture);
                    } else {
                        hud.update_weapon_sprite_hud(Gd::null_arg());
                        godot_warn!("Weapon texture not found for: {}", weapon_name);
                    }
                    if weapon_index != self.current_weapon_index {
                        self.camera.set_zoom(Vector2::new(1.0, 1.0));
                        weapon.bind_mut().deploy();
                    }
                    weapon.bind_mut().weapon_ready();
                    // 更新HUD
                    weapon.bind_mut().update_ammo_hud();
                    hud.update_speed_hud(self.current_speed);
                    hud.update_damage_hud(weapon.bind().get_damage(), self.damage);
                    hud.update_distance_hud(weapon.bind().get_distance(), self.distance);
                    hud.update_repel_hud(weapon.bind().get_repel(), self.repel);
                    hud.update_penetrate_hud(weapon.bind().get_penetrate(), self.penetrate);
                    weapon.bind().update_jitter_hud();
                } else {
                    weapon.set_visible(false);
                    // 打断其他武器的换弹
                    weapon.bind_mut().stop_reload();
                }
            }
        }
        self.state = PlayerState::Guard;
        self.guard();
        if weapon_index == self.current_weapon_index {
            self.change_fail_audio.play();
            return;
        }
        self.animated_sprite2d.play_ex().name("guard").done();
        self.current_speed = self.speed * 0.75;
        self.current_weapon_index = weapon_index;
        self.change_success_audio.play();
    }

    pub fn update_laser(&mut self) {
        if self.ray_cast2d.is_colliding() {
            let point = self
                .ray_cast2d
                .get_collision_point()
                .distance_to(self.ray_cast2d.get_global_position());
            self.line2d.set_point_position(1, Vector2::new(point, 0.0));
        } else {
            self.line2d
                .set_point_position(1, self.ray_cast2d.get_target_position());
        }
    }

    #[func]
    pub fn zoom(&mut self) {
        let weapon_name = self.get_current_weapon().get_name().to_string();
        if weapon_name == "M95" || weapon_name == "SKULL-5" {
            self.zoom_audio.play();
            self.camera.set_zoom(Vector2::new(0.5, 0.5));
        } else if weapon_name == "AWP" || weapon_name == "SKULL-6" {
            self.zoom_audio.play();
            self.camera.set_zoom(Vector2::new(0.65, 0.65));
        } else if weapon_name == "AK47-60R" || weapon_name == "M32" {
            self.zoom_audio.play();
            self.camera.set_zoom(Vector2::new(0.8, 0.8));
        } else if weapon_name == "RPG-7" {
            self.zoom_audio.play();
            self.camera.set_zoom(Vector2::new(0.84, 0.84));
        }
    }

    pub fn get_current_weapon(&self) -> Gd<RustWeapon> {
        self.weapons
            .get_child(self.current_weapon_index)
            .expect("Weapon not configured")
            .cast::<RustWeapon>()
    }

    // 消音武器
    #[func]
    pub fn unlock_usp(&mut self) {
        self.unlock_weapon("usp", 0, "1");
    }

    #[func]
    pub fn unlock_deagle(&mut self) {
        self.unlock_weapon("deagle", 1, "2");
    }

    #[func]
    pub fn unlock_m1887(&mut self) {
        self.unlock_weapon("m1887", 2, "3");
    }

    #[func]
    pub fn unlock_awp(&mut self) {
        self.unlock_weapon("awp", 3, "4");
    }

    // 强力武器
    #[func]
    pub fn unlock_m79(&mut self) {
        self.unlock_weapon("m79", 4, "5");
    }

    // 消音武器
    #[func]
    pub fn unlock_m4a1(&mut self) {
        self.unlock_weapon("m4a1", 5, "6");
    }

    #[func]
    pub fn unlock_ak47(&mut self) {
        self.unlock_weapon("ak47", 6, "7");
    }

    #[func]
    pub fn unlock_xm1014(&mut self) {
        self.unlock_weapon("xm1014", 7, "8");
    }

    // 强力武器
    #[func]
    pub fn unlock_ak47_60r(&mut self) {
        self.unlock_weapon("ak47-60r", 8, "9");
    }

    #[func]
    pub fn unlock_rpg_7(&mut self) {
        self.unlock_weapon("rpg-7", 9, "[(MAYBE MANY TIMES)");
    }

    #[func]
    pub fn unlock_m249(&mut self) {
        self.unlock_weapon("m249", 10, "[(MAYBE MANY TIMES)");
    }

    #[func]
    pub fn unlock_mg3(&mut self) {
        self.unlock_weapon("mg3", 11, "[(MAYBE MANY TIMES)");
    }

    // 强力武器
    #[func]
    pub fn unlock_skull_6(&mut self) {
        self.unlock_weapon("skull-6", 12, "[(MAYBE MANY TIMES)");
    }

    #[func]
    pub fn unlock_m95(&mut self) {
        self.unlock_weapon("m95", 13, "[(MAYBE MANY TIMES)");
    }

    #[func]
    pub fn unlock_m134(&mut self) {
        self.unlock_weapon("m134", 14, "[(MAYBE MANY TIMES)");
    }

    #[func]
    pub fn unlock_m32(&mut self) {
        self.unlock_weapon("m32", 15, "[(MAYBE MANY TIMES)");
    }

    // 强力武器
    #[func]
    pub fn unlock_xm1134(&mut self) {
        self.unlock_weapon("xm1134", 16, "[(MAYBE MANY TIMES)");
    }

    // 强力武器
    #[func]
    pub fn unlock_skull_5(&mut self) {
        self.unlock_weapon("skull-5", 17, "[(MAYBE MANY TIMES)");
    }

    pub fn unlock_weapon(&mut self, weapon_name: &str, index: i32, key: &str) {
        if self.weapons.get_child_count() > index {
            return;
        }
        if let Some(weapon) =
            load::<PackedScene>(&format!("res://scenes/weapons/{}.tscn", weapon_name))
                .try_instantiate_as::<RustWeapon>()
        {
            self.weapons.add_child(&weapon);
            if let Some(mut unlock_label) = self.create_message() {
                unlock_label.bind_mut().show_message(&format!(
                    "WEAPON {} UNLOCKED, PRESS {} TO USE IT",
                    weapon_name.to_uppercase(),
                    key
                ));
            }
            self.change_weapon(index);
        }
    }
}
