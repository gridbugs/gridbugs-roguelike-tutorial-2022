use crate::game::{Config, Game, Layer, Tile, VisibleCellData, VisibleEntityData};
use gridbugs::{
    chargrid::{control_flow::*, prelude::*},
    coord_2d::Size,
    direction::CardinalDirection,
    rgb_int::{Rgb24, Rgba32},
    visible_area_detection::CellVisibility,
};

// An update to the game state
enum GameAction {
    Move(CardinalDirection),
}

// Associate game actions with input events
fn game_action_from_input(input: Input) -> Option<GameAction> {
    match input {
        Input::Keyboard(keyboard_input) => {
            use CardinalDirection::*;
            use GameAction::*;
            match keyboard_input {
                KeyboardInput::Left => Some(Move(West)),
                KeyboardInput::Right => Some(Move(East)),
                KeyboardInput::Up => Some(Move(North)),
                KeyboardInput::Down => Some(Move(South)),
                _ => None,
            }
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct LightBlend {
    light_colour: Rgb24,
}

impl Tint for LightBlend {
    fn tint(&self, rgba32: Rgba32) -> Rgba32 {
        rgba32
            .to_rgb24()
            .normalised_mul(self.light_colour)
            .saturating_add(self.light_colour.saturating_scalar_mul_div(1, 10))
            .to_rgba32(255)
    }
}

// The state of the game
struct GameData {
    game: Game,
}

impl GameData {
    fn new(screen_size: Size, config: Config) -> Self {
        let game = Game::new(screen_size, config);
        Self { game }
    }

    // Update the game state by applying a game action
    fn handle_game_action(&mut self, game_action: GameAction) {
        match game_action {
            GameAction::Move(direction) => self.game.move_player(direction),
        }
    }

    // Associate each tile with a description of how to render it
    fn render_cell_from_entity_data(
        &self,
        visible_entity_data: &VisibleEntityData,
        coord: Coord,
    ) -> RenderCell {
        match visible_entity_data.tile {
            Tile::Player => RenderCell::BLANK.with_character('@').with_bold(true),
            Tile::Wall => {
                let is_wall_below = self.game.is_wall_known_at(coord + Coord::new(0, 1));
                if is_wall_below {
                    RenderCell::BLANK
                        .with_character(' ')
                        .with_background(Rgba32::new_grey(255))
                } else {
                    RenderCell::BLANK
                        .with_character('▄')
                        .with_background(Rgba32::new_grey(255))
                        .with_foreground(Rgba32::new_grey(127))
                }
            }
            Tile::DoorClosed => RenderCell::BLANK
                .with_character('+')
                .with_background(Rgba32::new_grey(127))
                .with_foreground(Rgba32::new_grey(255)),
            Tile::DoorOpen => RenderCell::BLANK
                .with_character('-')
                .with_background(Rgba32::new_grey(127))
                .with_foreground(Rgba32::new_grey(255)),
            Tile::Floor => RenderCell::BLANK
                .with_character('.')
                .with_foreground(Rgba32::new_grey(127)),
        }
    }

    fn layer_depth(layer: Layer) -> i8 {
        match layer {
            Layer::Character => 2,
            Layer::Feature => 1,
            Layer::Floor => 0,
        }
    }

    fn render_entity_data(
        &self,
        coord: Coord,
        visible_entity_data: &VisibleEntityData,
        layer: Layer,
        ctx: Ctx,
        fb: &mut FrameBuffer,
    ) {
        let render_cell = self.render_cell_from_entity_data(visible_entity_data, coord);
        let depth = Self::layer_depth(layer);
        fb.set_cell_relative_to_ctx(ctx, coord, depth, render_cell);
    }

    fn render_cell(&self, coord: Coord, cell: &VisibleCellData, ctx: Ctx, fb: &mut FrameBuffer) {
        cell.entity_data
            .option_for_each_enumerate(|visible_entity_data, layer| {
                self.render_entity_data(coord, visible_entity_data, layer, ctx, fb);
            });
    }

    fn render(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        for (coord, visibility) in self.game.enumerate_cell_visibility() {
            match visibility {
                CellVisibility::Never => (),
                CellVisibility::Previous(data)
                | CellVisibility::Current {
                    data,
                    light_colour: None,
                } => {
                    let dim_ctx = ctx.with_tint(&|colour: Rgba32| {
                        Rgba32::new_grey(colour.to_rgb24().max_channel() / 3)
                    });
                    self.render_cell(coord, data, dim_ctx, fb);
                }
                CellVisibility::Current {
                    data,
                    light_colour: Some(light_colour),
                } => {
                    let tint = LightBlend { light_colour };
                    let blend_ctx = ctx.with_tint(&tint);
                    self.render_cell(coord, data, blend_ctx, fb);
                }
            }
        }
    }
}

// A named unit type representing the renderable, interactive  game area
struct GameComponent;

impl Component for GameComponent {
    type Output = ();
    type State = GameData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        state.render(ctx, fb);
    }

    fn update(&mut self, state: &mut Self::State, _ctx: Ctx, event: Event) -> Self::Output {
        if let Event::Input(input) = event {
            if let Some(KeyboardInput::Char('r')) = input.keyboard() {
                state.game.reset();
            }
            if let Some(game_action) = game_action_from_input(input) {
                state.handle_game_action(game_action);
            }
        }
    }

    fn size(&self, _state: &Self::State, ctx: Ctx) -> Size {
        // The game will take up the entire window
        ctx.bounding_box.size()
    }
}

pub fn app(config: Config) -> App {
    // Instantiate the game state
    let screen_size = Size::new(60, 45);
    let game_data = GameData::new(screen_size, config);
    cf(GameComponent)
        .ignore_output() // Coerce the component's output type to `app::Output`.
        .with_state(game_data) // Associate the game state with the component.
        .exit_on_close() // Exit the program when its window is closed.
        .catch_escape() // Catch the escape event so we can exit on escape.
        .map(|res| match res {
            Err(Escape) => app::Exit, // Exit the program when escape is pressed.
            Ok(output) => output,     // Other outputs are simply returned.
        })
        .clear_each_frame()
}
