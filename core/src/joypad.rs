use sdl2::keyboard::Keycode;

/// Represents the state of the Game Boy's buttons.
/// True = pressed, False = released (internal representation)
#[derive(Clone, Debug, Default)]
pub struct JoypadState {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
}

/// Manages Joypad state and interaction with the P1 register.
#[derive(Clone, Debug, Default)]
pub struct Joypad {
    state: JoypadState,
    // Store the P1 register's selectable bits (written by the game)
    p1_register_selection: u8, // Bits 4 and 5
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            state: JoypadState::default(),
            // P1 defaults to 0xCF (often, depends post-bootrom),
            // means bits 4 and 5 are high (no selection) initially.
            // Store only the writable bits 4,5.
            p1_register_selection: 0x30,
        }
    }

    /// Reads the P1 (Joypad) register based on current state and selection.
    pub fn read_p1(&self) -> u8 {
        let mut joypad_value = 0x0F; // Start with lower nibble high (released)

        if self.p1_register_selection & 0x20 == 0 {
            // Bit 5 Low: Select Action buttons (A, B, Select, Start)
            if self.state.a {
                joypad_value &= 0b1110;
            } // Bit 0 low if pressed
            if self.state.b {
                joypad_value &= 0b1101;
            } // Bit 1 low if pressed
            if self.state.select {
                joypad_value &= 0b1011;
            } // Bit 2 low if pressed
            if self.state.start {
                joypad_value &= 0b0111;
            } // Bit 3 low if pressed
        }
        if self.p1_register_selection & 0x10 == 0 {
            // Bit 4 Low: Select Direction buttons (Right, Left, Up, Down)
            if self.state.right {
                joypad_value &= 0b1110;
            } // Bit 0 low if pressed
            if self.state.left {
                joypad_value &= 0b1101;
            } // Bit 1 low if pressed
            if self.state.up {
                joypad_value &= 0b1011;
            } // Bit 2 low if pressed
            if self.state.down {
                joypad_value &= 0b0111;
            } // Bit 3 low if pressed
        }

        // Combine input bits (0-3) with selection bits (4-5) and unused high bits (reads 1)
        joypad_value | self.p1_register_selection | 0xC0
    }

    /// Writes to the P1 (Joypad) register (only bits 4, 5 are writable).
    pub fn write_p1(&mut self, value: u8) {
        // Only bits 4 and 5 are writable
        self.p1_register_selection = value & 0x30;
    }

    /// Handles a key press event. Returns true if a Joypad interrupt should be requested.
    pub fn key_down(&mut self, key: Keycode) -> bool {
        let mut button_newly_pressed = false;
        let mut selection_active = false;

        match key {
            // Directions (Check bit 4 of P1 register selection)
            Keycode::Right | Keycode::D => {
                if !self.state.right {
                    button_newly_pressed = true;
                    self.state.right = true;
                }
                if self.p1_register_selection & 0x10 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Left | Keycode::A => {
                // Remap 'A' key to Left
                if !self.state.left {
                    button_newly_pressed = true;
                    self.state.left = true;
                }
                if self.p1_register_selection & 0x10 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Up | Keycode::W => {
                if !self.state.up {
                    button_newly_pressed = true;
                    self.state.up = true;
                }
                if self.p1_register_selection & 0x10 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Down | Keycode::S => {
                if !self.state.down {
                    button_newly_pressed = true;
                    self.state.down = true;
                }
                if self.p1_register_selection & 0x10 == 0 {
                    selection_active = true;
                }
            }
            // Actions (Check bit 5 of P1 register selection)
            Keycode::Z | Keycode::J => {
                // GB 'A' button
                if !self.state.a {
                    button_newly_pressed = true;
                    self.state.a = true;
                }
                if self.p1_register_selection & 0x20 == 0 {
                    selection_active = true;
                }
            }
            Keycode::X | Keycode::K => {
                // GB 'B' button
                if !self.state.b {
                    button_newly_pressed = true;
                    self.state.b = true;
                }
                if self.p1_register_selection & 0x20 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Backspace | Keycode::RShift => {
                // GB 'Select' button
                if !self.state.select {
                    button_newly_pressed = true;
                    self.state.select = true;
                }
                if self.p1_register_selection & 0x20 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Return | Keycode::Space => {
                // GB 'Start' button
                if !self.state.start {
                    button_newly_pressed = true;
                    self.state.start = true;
                }
                if self.p1_register_selection & 0x20 == 0 {
                    selection_active = true;
                }
            }
            _ => {} // Ignore other keys
        }

        // Request Joypad interrupt only if a button state changed from released->pressed
        // AND that button's group (Directions/Actions) is currently selected by the game.
        button_newly_pressed && selection_active
    }

    /// Handles a key release event.
    pub fn key_up(&mut self, key: Keycode) {
        match key {
            Keycode::Right | Keycode::D => self.state.right = false,
            Keycode::Left | Keycode::A => self.state.left = false,
            Keycode::Up | Keycode::W => self.state.up = false,
            Keycode::Down | Keycode::S => self.state.down = false,
            Keycode::Z | Keycode::J => self.state.a = false,
            Keycode::X | Keycode::K => self.state.b = false,
            Keycode::Backspace | Keycode::RShift => self.state.select = false,
            Keycode::Return | Keycode::Space => self.state.start = false,
            _ => {} // Ignore other keys
        }
    }

    // Optional: Allow external access to raw state if needed elsewhere
    pub fn get_state(&self) -> &JoypadState {
        &self.state
    }
}
