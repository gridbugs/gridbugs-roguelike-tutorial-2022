use gridbugs::{
    coord_2d::{Coord, Size},
    direction::{CardinalDirection, Direction},
    entity_table::{self, entity_data, entity_update, Entity, EntityAllocator},
    rgb_int::Rgb24,
    spatial_table,
    visible_area_detection::{
        vision_distance, CellVisibility, Light, Rational, VisibilityGrid, World as VisibleWorld,
    },
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

impl Tile {
    pub fn is_wall(&self) -> bool {
        match self {
            Self::Wall | Self::DoorClosed | Self::DoorOpen => true,
            _ => false,
        }
    }
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
        opacity: u8,
        light: Light<vision_distance::Circle>,
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
use layers::{LayerTable, Layers};
type SpatialTable = spatial_table::SpatialTable<Layers>;
type Location = spatial_table::Location<Layer>;

const PLAYER_VISION_DISTANCE: vision_distance::Circle = vision_distance::Circle::new_squared(1000);

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
                light: Light {
                    colour: Rgb24::new_grey(255),
                    vision_distance: PLAYER_VISION_DISTANCE,
                    diminish: Rational { numerator: 1, denominator: 150 },
                }
            },
        )
    }

    pub fn spawn_wall(&mut self, coord: Coord) {
        self.spawn_entity(
            (coord, Layer::Feature),
            entity_data! {
                tile: Tile::Wall,
                solid: (),
                opacity: 255,
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
                opacity: 255,
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

impl VisibleWorld for World {
    type VisionDistance = vision_distance::Circle;

    fn size(&self) -> Size {
        self.spatial_table.grid_size()
    }

    fn get_opacity(&self, coord: Coord) -> u8 {
        if let Some(&Layers {
            feature: Some(feature_entity),
            ..
        }) = self.spatial_table.layers_at(coord)
        {
            self.components
                .opacity
                .get(feature_entity)
                .cloned()
                .unwrap_or(0)
        } else {
            0
        }
    }
    fn for_each_light_by_coord<F: FnMut(Coord, &Light<Self::VisionDistance>)>(&self, mut f: F) {
        for (entity, light) in self.components.light.iter() {
            if let Some(coord) = self.spatial_table.coord_of(entity) {
                f(coord, light);
            }
        }
    }
}

pub struct VisibleEntityData {
    pub tile: Tile,
}

#[derive(Default)]
pub struct VisibleCellData {
    pub entity_data: LayerTable<Option<VisibleEntityData>>,
}

impl VisibleCellData {
    fn update(&mut self, world: &World, coord: Coord) {
        let layers = world.spatial_table.layers_at_checked(coord);
        self.entity_data = layers.option_and_then(|&entity| {
            let maybe_tile = world.components.tile.get(entity).cloned();
            maybe_tile.map(|tile| VisibleEntityData { tile })
        });
    }
}

// Level representation produced by terrain generation
struct Terrain {
    world: World,
    player_entity: Entity,
}

impl Terrain {
    fn generate(world_size: Size) -> Self {
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
}

pub struct Config {
    pub omniscient: bool,
}

// The state of the game
pub struct Game {
    world: World,
    player_entity: Entity,
    visibility_grid: VisibilityGrid<VisibleCellData>,
    config: Config,
}

impl Game {
    pub fn new(world_size: Size, config: Config) -> Self {
        let Terrain {
            world,
            player_entity,
        } = Terrain::generate(world_size);
        let visibility_grid = VisibilityGrid::new(world_size);
        let mut self_ = Self {
            world,
            player_entity,
            visibility_grid,
            config,
        };
        self_.update_visibility();
        self_
    }

    fn update_visibility(&mut self) {
        let update_fn = |data: &mut VisibleCellData, coord| data.update(&self.world, coord);

        if self.config.omniscient {
            self.visibility_grid.update_omniscient_custom(
                Rgb24::new_grey(255),
                &self.world,
                update_fn,
            );
        } else {
            let player_coord = self.get_player_coord();
            self.visibility_grid.update_custom(
                Rgb24::new_grey(0),
                &self.world,
                PLAYER_VISION_DISTANCE,
                player_coord,
                update_fn,
            );
        }
    }

    fn open_door(&mut self, entity: Entity) {
        self.world.components.apply_entity_update(
            entity,
            entity_update! {
                door_state: Some(DoorState::Open),
                tile: Some(Tile::DoorOpen),
                solid: None,
                opacity: None,
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
                opacity: 255,
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

    // Try to the player character one cell in the given direction. This may fail, or cause an
    // alternative action to happen, such as opening or closing doors.
    fn try_move_player(&mut self, direction: CardinalDirection) {
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

    // Move the player character one cell in the given direction
    pub fn move_player(&mut self, direction: CardinalDirection) {
        self.try_move_player(direction);
        self.update_visibility();
    }

    // Returns an iterator over each coordinate of the world, along with the visibility of each
    // corresponding cell
    pub fn enumerate_cell_visibility(
        &self,
    ) -> impl Iterator<Item = (Coord, CellVisibility<&VisibleCellData>)> {
        self.visibility_grid.enumerate()
    }

    /// Returns true iff a wall has been seen by the player at the given coord
    pub fn is_wall_known_at(&self, coord: Coord) -> bool {
        if let Some(data) = self.visibility_grid.get_data(coord) {
            data.entity_data
                .feature
                .as_ref()
                .map(|entity_data| entity_data.tile.is_wall())
                .unwrap_or(false)
        } else {
            false
        }
    }
}
