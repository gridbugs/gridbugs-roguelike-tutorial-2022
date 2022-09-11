use gridbugs::{
    coord_2d::{Coord, Size},
    direction::CardinalDirection,
    entity_table::{self, Entity, EntityAllocator},
};

#[derive(Clone, Copy, Debug)]
pub enum Tile {
    Player,
}

// Generates the type for a database storing entities. Each field is a table which maps an `Entity`
// (just a unique identifier) to a component value.
// E.g.
// struct Components {
//   coord: ComponentTable<Coord>,
//   tile: ComponentTable<Tile>,
//   ...
// }
// A `ComponentTable<T>` maps entities to values of type `T`.
entity_table::declare_entity_module! {
    components {
        coord: Coord,
        tile: Tile,
    }
}
use components::Components;

// The state of the game's world
#[derive(Default)]
pub struct World {
    components: Components, // the components of each entity in the world
    entity_allocator: EntityAllocator, // used to allocate new entities
}

impl World {
    // Add a new entity representing the player character at the given coord
    pub fn spawn_player(&mut self, coord: Coord) -> Entity {
        let entity = self.entity_allocator.alloc();
        self.components.coord.insert(entity, coord);
        self.components.tile.insert(entity, Tile::Player);
        entity
    }
}

// Information needed to render an entity
pub struct EntityToRender {
    pub coord: Coord,
    pub tile: Tile,
}

// The state of the game
pub struct Game {
    world: World,
    player_entity: Entity,
    world_size: Size,
}

impl Game {
    pub fn new(world_size: Size) -> Self {
        let mut world = World::default();
        let centre = world_size.to_coord().unwrap() / 2;
        // The player starts in the centre of the screen
        let player_entity = world.spawn_player(centre);
        Self {
            world,
            player_entity,
            world_size,
        }
    }

    // Returns the coordinate of the player character
    fn get_player_coord(&self) -> Coord {
        *self
            .world
            .components
            .coord
            .get(self.player_entity)
            .expect("player does not have coord")
    }

    // Move the player character one cell in the given direction
    pub fn move_player(&mut self, direction: CardinalDirection) {
        let player_coord = self.get_player_coord();
        let new_player_coord = player_coord + direction.coord();
        // Don't let the player walk off the screen
        if new_player_coord.is_valid(self.world_size) {
            self.world
                .components
                .coord
                .insert(self.player_entity, new_player_coord);
        }
    }

    // Returns an iterator over rendering information for each renderable entity
    pub fn entities_to_render(&self) -> impl '_ + Iterator<Item = EntityToRender> {
        self.world
            .components
            .tile
            .iter()
            .filter_map(|(entity, &tile)| {
                let &coord = self.world.components.coord.get(entity)?;
                Some(EntityToRender { tile, coord })
            })
    }
}
