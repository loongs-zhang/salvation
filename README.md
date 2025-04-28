# salvation

## How to run this project

1.install godot
```shell
# --- Linux ---
# For Ubuntu or Debian-based distros.
apt install godot

# For Fedora/RHEL.
dnf install godot

# Distro-independent through Flatpak.
flatpak install flathub org.godotengine.Godot


# --- Windows ---
# Windows installations can be made through WinGet.
winget install --id=GodotEngine.GodotEngine -e


# --- macOS ---
brew install godot
```
2.install rust
```shell
# Linux (distro-independent)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
winget install --id=Rustlang.Rustup -e

# macOS
brew install rustup
```
3.build lib
```shell
cargo build
```
4.run project in godot