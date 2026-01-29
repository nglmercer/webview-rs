use std::time::{Duration, Instant};
use std::env;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use pixels::{Pixels, SurfaceTexture};

fn main() {
    // --- FIX AUTOMÁTICO ---
    // En Linux, forzamos el backend X11 internamente.
    // Esto hace que la app use XWayland automáticamente si el usuario está en Wayland,
    // evitando el "Error 71 Protocol Error" sin que el usuario configure nada.
    #[cfg(target_os = "linux")]
    {
        env::set_var("GDK_BACKEND", "x11");
    }

    let event_loop = EventLoop::new();
    
    let window_inner = WindowBuilder::new()
        .with_title("Rust Render Test - Color Cycle")
        .with_inner_size(tao::dpi::LogicalSize::new(400.0, 300.0))
        .build(&event_loop)
        .unwrap();

    // Referencia estática (tu truco del Box::leak)
    let window = Box::leak(Box::new(window_inner));
    // Importante: Convertir a referencia compartida para que sea Copy
    let window = &*window; 

    let window_size = window.inner_size();
    
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
    let mut pixels = Pixels::new(window_size.width, window_size.height, surface_texture).unwrap();

    let colors = [
        (255, 0, 0),     // Red
        (0, 255, 0),     // Green
        (0, 0, 255),     // Blue
        (255, 255, 255), // White
        (0, 0, 0),       // Black
    ];

    let mut current_color_index = 0;
    let mut last_update = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // Usamos Wait en lugar de Poll para no quemar CPU esperando eventos
        *control_flow = ControlFlow::Wait;

        match event {
            // Evento: La ventana pide dibujarse
            Event::RedrawRequested(_) => {
                let (r, g, b) = colors[current_color_index];
                let frame = pixels.frame_mut();
                
                for pixel in frame.chunks_exact_mut(4) {
                    pixel[0] = r;
                    pixel[1] = g;
                    pixel[2] = b;
                    pixel[3] = 255;
                }

                if let Err(err) = pixels.render() {
                    eprintln!("Error render: {}", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                // Lógica de cambio de color (Timer)
                if last_update.elapsed() >= Duration::from_secs(1) {
                    current_color_index = (current_color_index + 1) % colors.len();
                    last_update = Instant::now();
                    println!("Cambiando color...");
                }
                
                // Solicitamos el siguiente frame inmediatamente para mantener la animación viva
                window.request_redraw();
            },

            // Evento: Se cambió el tamaño de la ventana (CRÍTICO para que no crashee al maximizar)
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                if size.width > 0 && size.height > 0 {
                    if let Err(err) = pixels.resize_surface(size.width, size.height) {
                        eprintln!("Error resizing surface: {}", err);
                        *control_flow = ControlFlow::Exit;
                    }
                    if let Err(err) = pixels.resize_buffer(size.width, size.height) {
                        eprintln!("Error resizing buffer: {}", err);
                        *control_flow = ControlFlow::Exit;
                    }
                    window.request_redraw();
                }
            },

            // Evento: Cerrar ventana
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            },

            // Evento: Loop idle (para arrancar el redibujado inicial)
            Event::MainEventsCleared => {
                 window.request_redraw();
            }
            
            _ => ()
        }
    });
}