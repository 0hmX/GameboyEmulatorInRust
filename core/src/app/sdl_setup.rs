use crate::constants;
use sdl2::Sdl;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::ttf::Sdl2TtfContext; // Keep Path for checking
use sdl2::video::{Window, WindowContext};

// No lifetime parameter needed
pub struct SdlContext {
    pub sdl: Sdl,
    pub ttf_context: Sdl2TtfContext,
    pub canvas: Canvas<Window>,
    pub texture_creator: TextureCreator<WindowContext>,
    pub event_pump: sdl2::EventPump,
}

// No lifetime parameter needed in signature or return type
pub fn init_sdl(window_title: &str) -> Result<SdlContext, String> {
    println!("Initializing SDL2...");
    let sdl = sdl2::init()?;
    let video_subsystem = sdl.video()?;

    println!("Initializing SDL2_ttf...");
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    // --- FONT LOADING REMOVED FROM HERE ---
    // println!("Loading font: {}...", font_path_str);
    // let font_path = Path::new(font_path_str);
    // if !font_path.exists() {
    //     return Err(format!("Font file not found: {}", font_path_str));
    // }
    // let font = ttf_context.load_font(font_path, constants::DEBUG_FONT_SIZE)?;
    // println!("Font loaded successfully.");

    let (window_width, window_height) = constants::calculate_window_dims();
    println!("Creating window ({}x{})...", window_width, window_height);

    let window = video_subsystem
        .window(window_title, window_width, window_height)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    println!("Creating accelerated canvas...");
    let canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();

    println!("Initializing event pump...");
    let event_pump = sdl.event_pump()?;

    // Return struct *without* the font
    Ok(SdlContext {
        sdl,
        ttf_context, // Move the context
        canvas,
        texture_creator,
        // font, // No font field anymore
        event_pump,
    })
}
