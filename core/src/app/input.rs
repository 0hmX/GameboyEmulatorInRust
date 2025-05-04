use boba::memory_bus::MemoryBus;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

/// Polls SDL events and updates the MemoryBus joypad state.
/// Returns `true` if the quit event was received, `false` otherwise.
pub fn handle_input(event_pump: &mut EventPump, memory_bus: &mut MemoryBus) -> bool {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => {
                println!("Exit requested.");
                return true; // Signal quit
            }
            Event::KeyDown {
                keycode: Some(key),
                repeat: false,
                ..
            } => {
                memory_bus.key_down(key); // Delegate to MemoryBus
            }
            Event::KeyUp {
                keycode: Some(key),
                repeat: false,
                ..
            } => {
                memory_bus.key_up(key); // Delegate to MemoryBus
            }
            _ => {} // Ignore other events
        }
    }
    false // Continue running
}
