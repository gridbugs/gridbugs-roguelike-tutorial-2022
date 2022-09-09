use gridbugs::{
    chargrid::{control_flow::*, prelude::*},
    coord_2d::{Coord, Size},
    direction::CardinalDirection,
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
    player_coord: Coord,
    screen_size: Size,
}

impl GameData {
    fn new(screen_size: Size) -> Self {
        // The player starts in the centre of the screen
        let player_coord = screen_size.to_coord().unwrap() / 2;
        Self {
            player_coord,
            screen_size,
        }
    }

    // Move the player character one cell in the given direction
    fn move_player(&mut self, direction: CardinalDirection) {
        let new_player_coord = self.player_coord + direction.coord();
        // Don't let the player walk off the screen
        if new_player_coord.is_valid(self.screen_size) {
            self.player_coord = new_player_coord;
        }
    }

    // Update the game state by applying a game action
    fn handle_game_action(&mut self, game_action: GameAction) {
        match game_action {
            GameAction::Move(direction) => self.move_player(direction),
        }
    }
}

// A named unit type representing the renderable, interactive  game area
struct GameComponent;

impl Component for GameComponent {
    type Output = ();
    type State = GameData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        // The player will be represented with a bold '@' sign
        let render_cell_player = RenderCell::BLANK.with_character('@').with_bold(true);

        // Draw the player character to the frame buffer relative to the current context, which
        // allows this component to be nested inside other components.
        fb.set_cell_relative_to_ctx(ctx, state.player_coord, 0, render_cell_player);
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
