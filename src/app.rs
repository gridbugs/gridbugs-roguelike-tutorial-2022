use crate::game::{EntityToRender, Game, Tile};
use gridbugs::{
    chargrid::{control_flow::*, prelude::*},
    coord_2d::Size,
    direction::CardinalDirection,
    rgb_int::Rgba32,
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

// The state of the game
struct GameData {
    game: Game,
}

impl GameData {
    fn new(screen_size: Size) -> Self {
        let game = Game::new(screen_size);
        Self { game }
    }

    // Update the game state by applying a game action
    fn handle_game_action(&mut self, game_action: GameAction) {
        match game_action {
            GameAction::Move(direction) => self.game.move_player(direction),
        }
    }

    // Associate each tile with a description of how to render it
    fn render_cell_from_tile(&self, tile: Tile) -> RenderCell {
        match tile {
            Tile::Player => RenderCell::BLANK.with_character('@').with_bold(true),
            Tile::Wall => RenderCell::BLANK
                .with_character('#')
                .with_background(Rgba32::new_grey(255))
                .with_foreground(Rgba32::new_grey(0)),
        }
    }

    fn render(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        for EntityToRender { coord, tile } in self.game.entities_to_render() {
            let render_cell = self.render_cell_from_tile(tile);
            fb.set_cell_relative_to_ctx(ctx, coord, 0, render_cell);
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

pub fn app() -> App {
    // Instantiate the game state
    let screen_size = Size::new(60, 45);
    let game_data = GameData::new(screen_size);
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
