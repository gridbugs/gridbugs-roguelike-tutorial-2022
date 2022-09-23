use gridbugs::{
    coord_2d::{Coord, Size},
    direction::CardinalDirection,
    entity_table::{self, entity_data, Entity, EntityAllocator},
    spatial_table,
};

#[derive(Clone, Copy, Debug)]
pub enum Tile {
    Player,
    Wall,
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
        tile: Tile,
        solid: (),
    }
}
use components::{Components, EntityData};

spatial_table::declare_layers_module! {
    layers {
        character: Character,
        feature: Feature,
    }
}

use layers::{Layer, Layers};
type SpatialTable = spatial_table::SpatialTable<Layers>;
type Location = spatial_table::Location<Layer>;

// The state of the game's world
pub struct World {
    components: Components, // the components of each entity in the world
    entity_allocator: EntityAllocator, // used to allocate new entities
    spatial_table: SpatialTable,
}

impl World {
    pub fn new(size: Size) -> Self {
        let components = Components::default();
        let spatial_table = SpatialTable::new(size);
        let entity_allocator = EntityAllocator::default();
        Self {
            components,
            spatial_table,
            entity_allocator,
        }
    }

    // Helper method to spawn an entity at a location
    fn spawn_entity<L: Into<Location>>(&mut self, location: L, entity_data: EntityData) -> Entity {
        let entity = self.entity_allocator.alloc();
        let location @ Location { layer, coord } = location.into();
        if let Err(e) = self.spatial_table.update(entity, location) {
            panic!("{:?}: There is already a {:?} at {:?}", e, layer, coord);
        }
        self.components.insert_entity_data(entity, entity_data);
        entity
    }

    // Add a new entity representing the player character at the given coord
    pub fn spawn_player(&mut self, coord: Coord) -> Entity {
        self.spawn_entity(
            (coord, Layer::Character),
            entity_data! {
                tile: Tile::Player,
            },
        )
    }

    pub fn spawn_wall(&mut self, coord: Coord) {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::Wall,
                solid: (),
            },
        );
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
        let mut world = World::new(world_size);
        let centre = world_size.to_coord().unwrap() / 2;
        // The player starts in the centre of the screen
        let player_entity = world.spawn_player(centre);
        // Make a vertical section of wall to the east of the player
        for i in 0..=10 {
            let coord = centre + Coord::new(5, i - 5);
            world.spawn_wall(coord);
        }
        // Make a horizontal section of wall to the south of the player
        for i in 0..10 {
            let coord = centre + Coord::new(i - 5, 5);
            world.spawn_wall(coord);
        }
        Self {
            world,
            player_entity,
            world_size,
        }
    }

    // Returns the coordinate of the player character
    fn get_player_coord(&self) -> Coord {
        self.world
            .spatial_table
            .coord_of(self.player_entity)
            .expect("player does not have coord")
    }

    // Move the player character one cell in the given direction
    pub fn move_player(&mut self, direction: CardinalDirection) {
        let player_coord = self.get_player_coord();
        let new_player_coord = player_coord + direction.coord();
        // Don't let the player walk off the screen
        if new_player_coord.is_valid(self.world_size) {
            // Don't let the player walk through solid entities
            for solid_entity in self.world.components.solid.entities() {
                if let Some(solid_coord) = self.world.spatial_table.coord_of(solid_entity) {
                    if new_player_coord == solid_coord {
                        return;
                    }
                }
            }
            self.world
                .spatial_table
                .update_coord(self.player_entity, new_player_coord)
                .unwrap();
        }
    }

    // Returns an iterator over rendering information for each renderable entity
    pub fn entities_to_render(&self) -> impl '_ + Iterator<Item = EntityToRender> {
        self.world
            .components
            .tile
            .iter()
            .filter_map(|(entity, &tile)| {
                let coord = self.world.spatial_table.coord_of(entity)?;
                Some(EntityToRender { tile, coord })
            })
    }
}
