use gridbugs::{chargrid_ansi_terminal, chargrid_wgpu};

mod app;
mod game;
mod terrain;

// Command-line arguments
struct Args {
    terminal: bool,
    omniscient: bool,
    rng_seed: Option<u64>,
}

impl Args {
    pub fn parser() -> impl meap::Parser<Item = Self> {
        meap::let_map! {
            let {
                terminal = flag("terminal").desc("run in a terminal");
                omniscient = flag("omniscient").desc("give the player omniscient vision");
                rng_seed = opt_opt::<u64, _>("INT", "rng-seed").desc("rng seed");
            } in {
                Self {
                    terminal,
                    omniscient,
                    rng_seed,
                }
            }
        }
    }
}

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

fn main() {
    use meap::Parser;
    let Args {
        terminal,
        omniscient,
        rng_seed,
    } = Args::parser().with_help_default().parse_env_or_exit();
    let config = game::Config {
        omniscient,
        rng_seed,
    };
    let app = app::app(config);
    if terminal {
        // Run the app in an ANSI terminal chargrid context
        use chargrid_ansi_terminal::{Context, FromTermInfoRgb};
        let context = Context::new().expect("Failed to initialize terminal");
        let colour = FromTermInfoRgb; // Use 256-colour encoding
        context.run(app, colour);
    } else {
        // Run the app in a WGPU chargrid context
        let context = wgpu_context();
        context.run(app);
    }
}
