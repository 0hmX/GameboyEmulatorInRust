# Boba - A Game Boy Emulator in Rust

![Build Status](https://img.shields.io/badge/build-passing-brightgreen) <!-- Optional: Link to your CI build status -->
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)   <!-- Optional: Update with your chosen license -->

Boba is a work-in-progress Game Boy (DMG) emulator written in Rust. The primary goal of this project is educational: to learn about emulator development, the Game Boy hardware architecture, and low-level programming concepts using Rust.


https://github.com/user-attachments/assets/0690f1d7-404a-4f7d-b7a6-172b3d3388c4


## About The Project

This project aims to create a functional Game Boy emulator capable of playing classic Game Boy titles. It focuses on understanding and implementing the core components of the Game Boy:

*   **CPU:** Emulates the Sharp LR35902 processor (a hybrid between Z80 and 8080).
*   **PPU (Picture Processing Unit):** Handles graphics rendering, including backgrounds, sprites, and windows.
*   **APU (Audio Processing Unit):** Emulates the sound channels of the Game Boy (currently basic/placeholder).
*   **Memory Management:** Simulates the Game Boy's memory map and handles communication between components.
*   **Input:** Maps keyboard inputs to Game Boy button presses.

The emulator uses the SDL2 library for windowing, graphics rendering, input handling, and potentially audio output. It also includes basic debugging features.

## Features (Current / Planned)

*   Loads and executes Game Boy ROMs (`.gb` files).
*   CPU instruction emulation.
*   Basic PPU rendering (Backgrounds, Sprites).
*   Basic APU sound output (Work in Progress).
*   Keyboard input support.
*   Debug Views:
    *   VRAM Tile Viewer
    *   CPU Disassembly (Simple)
    *   Input State Display
*   Instruction Stepping Mode for debugging.
*   (Planned) Improved accuracy for CPU and PPU timing.
*   (Planned) Save States.
*   (Planned) Game Boy Color (CGB) support.

## Prerequisites

Before you can build and run Boba, you need the following installed:

1.  **Rust Toolchain:**
    *   Install Rust via [rustup](https://rustup.rs/). This will provide `rustc` (the compiler) and `cargo` (the build tool and package manager).
    *   `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

2.  **SDL2 Library:** Boba uses SDL2 for graphics, windowing, and input.
    *   **Ubuntu/Debian:** `sudo apt-get update && sudo apt-get install libsdl2-dev`
    *   **Fedora:** `sudo dnf install SDL2-devel`
    *   **Arch Linux:** `sudo pacman -S sdl2`
    *   **macOS (using Homebrew):** `brew install sdl2`
    *   **Windows:** Download the Development Libraries from the [SDL2 website](https://www.libsdl.org/download-2.0.php) and follow instructions for setting up your environment (often involves placing DLLs near the executable or setting PATH).

3.  **SDL2_ttf Library:** Used for rendering debug text.
    *   **Ubuntu/Debian:** `sudo apt-get install libsdl2-ttf-dev`
    *   **Fedora:** `sudo dnf install SDL2_ttf-devel`
    *   **Arch Linux:** `sudo pacman -S sdl2_ttf`
    *   **macOS (using Homebrew):** `brew install sdl2_ttf`
    *   **Windows:** Download the Development Libraries from the [SDL2_ttf website](https://www.libsdl.org/projects/SDL_ttf/) and set it up similarly to SDL2.

4.  **A Game Boy ROM file:** You will need a `.gb` ROM file to run the emulator. **ROM files are not provided with this project due to copyright.** Please use legally obtained ROMs (e.g., from homebrew games or backups of cartridges you own).

## Installation & Building

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/your_username/boba.git # Replace with your repo URL
    cd boba
    ```

2.  **Build the project:**
    *   For a debug build (faster compilation, slower execution):
        ```bash
        cargo build
        ```
    *   For a release build (slower compilation, optimized for speed):
        ```bash
        cargo build --release
        ```
    The executable will be located in `target/debug/boba` or `target/release/boba`.

## Running the Emulator

You can run the emulator using `cargo run` or by executing the compiled binary directly. You **must** provide the path to a Game Boy ROM file as a command-line argument.

**Using Cargo:**

```bash
cargo run --release -- <path/to/your/rom.gb>
```

*(Remove `--release` if you want to run a debug build)*

**Directly:**

```bash
./target/release/boba <path/to/your/rom.gb>
```
*(Use `target/debug/boba` for a debug build)*

### Controls

*   **D-Pad:** Arrow Keys
*   **A Button:** X
*   **B Button:** Z
*   **Start:** Enter
*   **Select:** Right Shift
*   **Toggle Pause/Step Mode:** P
*   **Next Instruction (when paused):** N
*   **Quit:** Escape (or closing the window)

*(Note: Verify and update these controls if they differ in your `input.rs` implementation)*

## Project Structure

The project is organized into a core library and an application binary:

```
boba/
├── Cargo.toml         # Project configuration and dependencies
├── LICENSE            # Project license file (ADD ONE!)
├── README.md          # This file
├── assets/            # Non-code assets (e.g., fonts)
│   └── fonts/
│       └── Roboto-Regular.ttf # Or DejaVuSansMono.ttf, etc.
├── src/
│   ├── lib.rs         # Core emulator library (defines boba::cpu, boba::ppu etc.)
│   ├── cpu.rs         # CPU emulation logic
│   ├── ppu.rs         # PPU emulation logic
│   ├── apu.rs         # APU emulation logic
│   ├── memory_bus.rs  # Memory mapping and bus logic
│   ├── joypad.rs      # Joypad state logic
│   ├── cartridge.rs   # (Optional) ROM loading/parsing logic
│   └── app/           # Application-specific code (SDL integration, UI)
│       ├── main.rs        # Binary entry point, main loop, event handling
│       ├── constants.rs   # UI and timing constants
│       ├── drawing.rs     # SDL drawing helper functions
│       ├── emulator.rs    # Emulator struct wrapping core components
│       ├── input.rs       # SDL input mapping logic
│       └── sdl_setup.rs   # SDL initialization logic
└── target/            # Build artifacts (created by cargo)
```

*(Note: Adapt this structure if your project layout differs, e.g., if `main.rs` is directly under `src/`)*

## License

This project is licensed under the terms of the MIT license and the Apache License (Version 2.0). Choose one or both.

**You should add a `LICENSE-MIT` and/or `LICENSE-APACHE` file to the root of your repository.**
NO USE ALLOWED FOR Ai without a paid licences!

See `LICENSE-APACHE` and `LICENSE-MIT` for details.

---

THIS CODE IS AN INTALECTURAL PROPERTY TO TRAIN LLM/AI ON THIS YOU NEED TO PAY ME 1K USD
