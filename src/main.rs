use gridbugs::{chargrid::control_flow::*, chargrid_wgpu};

// Create a context for running chargrid apps in a WGPU graphical window
fn wgpu_context() -> chargrid_wgpu::Context {
    use chargrid_wgpu::*;
    const CELL_SIZE_PX: f64 = 16.;
    Context::new(Config {
        font_bytes: FontBytes {
            normal: include_bytes!("./fonts/PxPlus_IBM_CGAthin.ttf").to_vec(),
            bold: include_bytes!("./fonts/PxPlus_IBM_CGA.ttf").to_vec(),
        },
        title: "Gridbugs Roguelike Tutorial".to_string(),
        window_dimensions_px: Dimensions {
            width: 960.,
            height: 720.,
        },
        cell_dimensions_px: Dimensions {
            width: CELL_SIZE_PX,
            height: CELL_SIZE_PX,
        },
        font_scale: Dimensions {
            width: CELL_SIZE_PX,
            height: CELL_SIZE_PX,
        },
        underline_width_cell_ratio: 0.1,
        underline_top_offset_cell_ratio: 0.8,
        resizable: false,
        force_secondary_adapter: false,
    })
}

// A placeholder chargrid app that displays the text "Hello, World!"
fn app() -> App {
    // Create a component which ignores its input and renders a string.
    styled_string("Hello, World!".to_string(), Default::default())
        .centre() // Display the text in the centre of the window.
        .ignore_output() // Coerce the component's output type to `app::Output`.
        .exit_on_close() // Terminate the program when the window is closed.
        .catch_escape() // Intercept the escape key so we can terminate on escape.
        .map(|res| match res {
            // Terminate the program when the escape key is pressed.
            Ok(app::Exit) | Err(Escape) => app::Exit,
        })
}

fn main() {
    // Create the WGPU chargrid context and run the app
    let context = wgpu_context();
    context.run(app());
}
