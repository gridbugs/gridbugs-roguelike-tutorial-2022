use gridbugs::{
    coord_2d::{Coord, Size},
    direction::{CardinalDirection, Direction},
    entity_table::{self, entity_data, entity_update, Entity, EntityAllocator},
    spatial_table,
};

#[derive(Clone, Copy, Debug)]
pub enum DoorState {
    Open,
    Closed,
}

#[derive(Clone, Copy, Debug)]
pub enum Tile {
    Player,
    Wall,
    DoorOpen,
    DoorClosed,
    Floor,
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
        door_state: DoorState,
    }
}
use components::{Components, EntityData, EntityUpdate};

spatial_table::declare_layers_module! {
    layers {
        character: Character,
        feature: Feature,
        floor: Floor,
    }
}

pub use layers::Layer;
use layers::Layers;
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

    pub fn spawn_door(&mut self, coord: Coord) {
        // Remove any existing feautures (e.g. walls) at this location
        if let &Layers {
            feature: Some(feature_entity),
            ..
        } = self.spatial_table.layers_at_checked(coord)
        {
            self.spatial_table.remove(feature_entity);
            self.components.remove_entity(feature_entity);
            self.entity_allocator.free(feature_entity);
        }
        // Add the door
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::DoorClosed,
                door_state: DoorState::Closed,
                solid: (),
            },
        );
    }

    pub fn spawn_floor(&mut self, coord: Coord) {
        self.spawn_entity(
            (coord, Layer::Floor),
            entity_data! {
                tile: Tile::Floor,
            },
        );
    }
}

// Information needed to render an entity
pub struct EntityToRender {
    pub coord: Coord,
    pub tile: Tile,
    pub layer: Layer,
}

// The state of the game
pub struct Game {
    world: World,
    player_entity: Entity,
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
        world.spawn_door(centre + Coord::new(5, 0));
        for coord in world_size.coord_iter_row_major() {
            world.spawn_floor(coord);
        }
        Self {
            world,
            player_entity,
        }
    }

    fn open_door(&mut self, entity: Entity) {
        self.world.components.apply_entity_update(
            entity,
            entity_update! {
                door_state: Some(DoorState::Open),
                tile: Some(Tile::DoorOpen),
                solid: None,
            },
        );
    }

    fn close_door(&mut self, entity: Entity) {
        self.world.components.insert_entity_data(
            entity,
            entity_data! {
                door_state: DoorState::Closed,
                tile: Tile::DoorClosed,
                solid: (),
            },
        );
    }

    fn open_door_entity_adjacent_to_coord(&self, coord: Coord) -> Option<Entity> {
        for direction in Direction::all() {
            let potential_door_coord = coord + direction.coord();
            if let Some(&Layers {
                feature: Some(feature_entity),
                ..
            }) = self.world.spatial_table.layers_at(potential_door_coord)
            {
                if let Some(DoorState::Open) = self.world.components.door_state.get(feature_entity)
                {
                    return Some(feature_entity);
                }
            }
        }
        None
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
        if let Some(&Layers {
            feature: Some(feature_entity),
            ..
        }) = self.world.spatial_table.layers_at(new_player_coord)
        {
            // If the player bumps into a door, open the door
            if let Some(DoorState::Closed) = self.world.components.door_state.get(feature_entity) {
                self.open_door(feature_entity);
                return;
            }
            // Don't let the player walk through solid entities
            if self.world.components.solid.contains(feature_entity) {
                if let Some(open_door_entity) =
                    self.open_door_entity_adjacent_to_coord(player_coord)
                {
                    self.close_door(open_door_entity);
                }
                return;
            }
        }
        self.world
            .spatial_table
            .update_coord(self.player_entity, new_player_coord)
            .unwrap();
    }

    // Returns an iterator over rendering information for each renderable entity
    pub fn entities_to_render(&self) -> impl '_ + Iterator<Item = EntityToRender> {
        self.world
            .components
            .tile
            .iter()
            .filter_map(|(entity, &tile)| {
                if let Some(&Location {
                    coord,
                    layer: Some(layer),
                }) = self.world.spatial_table.location_of(entity)
                {
                    Some(EntityToRender { tile, coord, layer })
                } else {
                    None
                }
            })
    }
}
