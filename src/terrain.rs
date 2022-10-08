use crate::game::World;
use gridbugs::{
    coord_2d::{Axis, Coord, Size},
    direction::{CardinalDirection, Direction},
    entity_table::Entity,
    grid_2d::Grid,
    perlin2::Perlin2,
};
use rand::{seq::SliceRandom, Rng};
use std::{collections::HashSet, mem};

// Will be used as cells in grids representing simple maps of levels during terrain generation
#[derive(Clone, Copy, PartialEq, Eq)]
enum FloorOrWall {
    Floor,
    Wall,
}

// An axis-aligned rectangle
#[derive(Clone, Copy)]
struct Rect {
    top_left: Coord,
    size: Size,
}

impl Rect {
    // Randomly generate a rectangle
    fn choose<R: Rng>(bounds: Size, min_size: Size, max_size: Size, rng: &mut R) -> Self {
        let width = rng.gen_range(min_size.width()..max_size.width());
        let height = rng.gen_range(min_size.height()..max_size.height());
        let size = Size::new(width, height);
        let top_left_bounds = bounds - size;
        let left = rng.gen_range(0..top_left_bounds.width());
        let top = rng.gen_range(0..top_left_bounds.height());
        let top_left = Coord::new(left as i32, top as i32);
        Self { top_left, size }
    }

    // Returns an iterator over all the coordinates in the rectangle
    fn coords(&self) -> impl '_ + Iterator<Item = Coord> {
        self.size.coord_iter_row_major().map(|c| c + self.top_left)
    }

    // Returns true iff the given coordinate is on the edge of the rectangle
    fn is_edge(&self, coord: Coord) -> bool {
        self.size.is_on_edge(coord - self.top_left)
    }

    // Returns an iterator over the edge coordinates of the rectangle
    fn edge_coords(&self) -> impl '_ + Iterator<Item = Coord> {
        self.size.edge_iter().map(|c| self.top_left + c)
    }

    // Returns an iterator over the internal (non-edge) coordinates of the rectangle
    fn internal_coords(&self) -> impl '_ + Iterator<Item = Coord> {
        self.coords().filter(|&c| !self.is_edge(c))
    }

    // Returns the coordinate of the centre of the rectangle
    fn centre(&self) -> Coord {
        self.top_left + (self.size / 2)
    }
}

// Represents a room during terrain generation
#[derive(Clone, Copy)]
struct Room {
    // The edge of the rectangle will be the walls surrounding the room, and the inside of the
    // rectangle will be the floor of the room.
    rect: Rect,
}

impl Room {
    // Returns true iff any cell of the room corresponds to a floor cell in the given map
    fn overlaps_with_floor(&self, map: &Grid<FloorOrWall>) -> bool {
        self.rect
            .coords()
            .any(|coord| *map.get_checked(coord) == FloorOrWall::Floor)
    }

    // Updates the given map, setting each cell corresponding to the floor of this room to be a
    // floor cell
    fn add_floor_to_map(&self, map: &mut Grid<FloorOrWall>) {
        for coord in self.rect.internal_coords() {
            *map.get_checked_mut(coord) = FloorOrWall::Floor;
        }
    }
}

// Checks whether a given cell of a map has a floor either side of it in the given axis, and a
// wall either side of it in the other axis. (An Axis is defined as `enum Axis { X, Y }`.)
// This is used to check whether a cell is suitable to contain a door.
fn is_cell_in_corridor_axis(map: &Grid<FloorOrWall>, coord: Coord, axis: Axis) -> bool {
    use FloorOrWall::*;
    let axis_delta = Coord::new_axis(1, 0, axis);
    let other_axis_delta = Coord::new_axis(0, 1, axis);
    let floor_in_axis = *map.get_checked(coord + axis_delta) == Floor
        && *map.get_checked(coord - axis_delta) == Floor;
    let wall_in_other_axis = *map.get_checked(coord + other_axis_delta) == Wall
        && *map.get_checked(coord - other_axis_delta) == Wall;
    floor_in_axis && wall_in_other_axis
}

// Checks whether a given cell of a map has a floor either side of it in some axis, and a wall
// either side of it in the other axis.
// This is used to check whether a cell is suitable to contain a door.
fn is_cell_in_corridor(map: &Grid<FloorOrWall>, coord: Coord) -> bool {
    is_cell_in_corridor_axis(map, coord, Axis::X) || is_cell_in_corridor_axis(map, coord, Axis::Y)
}

// Checks whether a cell has any neighbours which are floors
fn has_floor_neighbour(map: &Grid<FloorOrWall>, coord: Coord) -> bool {
    CardinalDirection::all().any(|d| *map.get_checked(coord + d.coord()) == FloorOrWall::Floor)
}

// Returns a vec of coordinates that define an L-shaped corridor from start to end (in order). The
// corridor stops if it encounters a cell adjacent to a floor cell according to the given map. The
// first axis that is traversed in the L-shaped corridor will be the given axis.
fn l_shaped_corridor_with_first_axis(
    start: Coord,
    end: Coord,
    map: &Grid<FloorOrWall>,
    first_axis: Axis,
) -> Vec<Coord> {
    let mut ret = Vec::new();
    let delta = end - start;
    let step = Coord::new_axis(delta.get(first_axis).signum(), 0, first_axis);
    // Skip the start coordinate so multiple corridors can start from the same coord
    let mut current = start + step;
    while current.get(first_axis) != end.get(first_axis) {
        ret.push(current);
        if has_floor_neighbour(map, current) {
            // stop when we get adjacent to a floor cell
            return ret;
        }
        current += step;
    }
    let step = Coord::new_axis(0, delta.get(first_axis.other()).signum(), first_axis);
    while current != end {
        ret.push(current);
        if has_floor_neighbour(map, current) {
            // stop when we get adjacent to a floor cell
            return ret;
        }
        current += step;
    }
    ret
}

// Returns a vec of coordinates that define an L-shaped corridor from start to end (in order). The
// corridor stops if it encounters a cell adjacent to a floor cell according to the given map. The
// first axis that is traversed in the L-shaped corridor is chosen at random.
fn l_shaped_corridor<R: Rng>(
    start: Coord,
    end: Coord,
    map: &Grid<FloorOrWall>,
    rng: &mut R,
) -> Vec<Coord> {
    let axis = if rng.gen() { Axis::X } else { Axis::Y };
    l_shaped_corridor_with_first_axis(start, end, map, axis)
}

// Data structure representing the state of the room-placement algorithm
struct RoomPlacement {
    // A list of rooms that have been placed
    rooms: Vec<Room>,
    // A set of all coordinates that are the edge of a room
    edge_coords: HashSet<Coord>,
    // List of cells that would be suitable to contain doors
    door_candidates: Vec<Coord>,
    // Tracks whether there is a wall or floor at each location
    map: Grid<FloorOrWall>,
}

impl RoomPlacement {
    fn new(size: Size) -> Self {
        Self {
            rooms: Vec::new(),
            edge_coords: HashSet::new(),
            door_candidates: Vec::new(),
            map: Grid::new_copy(size, FloorOrWall::Wall),
        }
    }

    // Adds a new room unless it overlaps with the floor
    fn try_add_room<R: Rng>(&mut self, new_room: Room, rng: &mut R) {
        // Don't add the room if it overlaps with the floor
        if new_room.overlaps_with_floor(&self.map) {
            return;
        }
        // Add the room's wall to the collection of edge coords
        self.edge_coords.extend(new_room.rect.edge_coords());
        // Randomly choose two rooms to connect the new room to
        for &existing_room in self.rooms.choose_multiple(rng, 2) {
            // List the coordinates of an L-shaped corridor between the centres of the new room and
            // the chosen exsiting room
            let corridor = l_shaped_corridor(
                new_room.rect.centre(),
                existing_room.rect.centre(),
                &self.map,
                rng,
            );
            // Carve out the corridor from the map
            for &coord in &corridor {
                *self.map.get_checked_mut(coord) = FloorOrWall::Floor;
            }
            // Update the list of door candidates along this corridor
            let mut door_candidate = None;
            for &coord in &corridor {
                if self.edge_coords.contains(&coord) && is_cell_in_corridor(&self.map, coord) {
                    door_candidate = Some(coord);
                } else if let Some(coord) = door_candidate.take() {
                    // The candidate is stored in door_candidate (an Option<Coord>) until a
                    // non-candidate cell is found, at which point the currently-stored candidate
                    // is added to the list of door candidates. This prevents multiple consecutive
                    // door candidates being added, which could result in several doors in a row
                    // which is undesired.
                    self.door_candidates.push(coord);
                }
            }
            if let Some(coord) = door_candidate {
                self.door_candidates.push(coord);
            }
        }
        new_room.add_floor_to_map(&mut self.map);
        self.rooms.push(new_room);
    }
}

// A cell of the RoomsAndCorridorsLevel map
#[derive(Clone, Copy, PartialEq, Eq)]
enum RoomsAndCorridorsCell {
    Floor,
    Wall,
    Door,
}

// Represents a level made up of rooms and corridors
struct RoomsAndCorridorsLevel {
    // Whether each cell is a floor or wall
    map: Grid<RoomsAndCorridorsCell>,
    // Location where the player will start
    player_spawn: Coord,
}

impl RoomsAndCorridorsLevel {
    // Randomly generates a level made up of rooms and corridors
    fn generate<R: Rng>(size: Size, rng: &mut R) -> Self {
        const NUM_ROOM_ATTEMPTS: usize = 50;
        const MIN_ROOM_SIZE: Size = Size::new_u16(5, 5);
        const MAX_ROOM_SIZE: Size = Size::new_u16(11, 9);
        let mut room_placement = RoomPlacement::new(size);
        // Add all the rooms and corridors
        for _ in 0..NUM_ROOM_ATTEMPTS {
            let new_room = Room {
                rect: Rect::choose(size, MIN_ROOM_SIZE, MAX_ROOM_SIZE, rng),
            };
            room_placement.try_add_room(new_room, rng);
        }
        // Create the map made of `RoomsAndCorridorsCell`s
        let mut map = Grid::new_grid_map(room_placement.map, |floor_or_wall| match floor_or_wall {
            FloorOrWall::Floor => RoomsAndCorridorsCell::Floor,
            FloorOrWall::Wall => RoomsAndCorridorsCell::Wall,
        });
        // Add doors
        for door_candidate_coord in room_placement.door_candidates {
            // Each door candidate has a 50% chance to become a door
            if rng.gen::<bool>() {
                *map.get_checked_mut(door_candidate_coord) = RoomsAndCorridorsCell::Door;
            }
        }
        // The player will start in the centre of a randomly-chosen room
        let player_spawn = room_placement.rooms.choose(rng).unwrap().rect.centre();
        Self { map, player_spawn }
    }
}

// Params for the Conway's Game of Life Cell Automata which will be used to generate caves
struct GameOfLifeParams {
    survive_min: u8,
    survive_max: u8,
    resurrect_min: u8,
    resurrect_max: u8,
}

// State for the Conway's Game of Life Cell Automata which will be used to generate caves
struct GameOfLife {
    alive: Grid<bool>,
    next: Grid<bool>,
}

impl GameOfLife {
    // Initialize state to random values
    fn new<R: Rng>(size: Size, rng: &mut R) -> Self {
        let alive = Grid::new_fn(size, |_| rng.gen::<bool>());
        let next = Grid::new_default(size);
        Self { alive, next }
    }

    // Progress the cell automata simulation by one step
    fn step(
        &mut self,
        &GameOfLifeParams {
            survive_min,
            survive_max,
            resurrect_min,
            resurrect_max,
        }: &GameOfLifeParams,
    ) {
        for ((coord, &alive_cell), next_cell) in self.alive.enumerate().zip(self.next.iter_mut()) {
            let n: u8 = Direction::all()
                .map(|direction| {
                    self.alive
                        .get(coord + direction.coord())
                        .cloned()
                        .unwrap_or(false) as u8
                })
                .sum();
            *next_cell = (alive_cell && n >= survive_min && n <= survive_max)
                || (!alive_cell && n >= resurrect_min && n <= resurrect_max);
        }
        mem::swap(&mut self.alive, &mut self.next);
    }
}

// Generate the starting point for the cave map by running a cell automata for several steps
fn generate_initial_cave_map<R: Rng>(size: Size, rng: &mut R) -> Grid<FloorOrWall> {
    const NUM_STEPS: usize = 10;
    let mut game_of_life = GameOfLife::new(size, rng);
    // This choice of params leads to cavernous regions of living cells
    let params = GameOfLifeParams {
        survive_min: 4,
        survive_max: 8,
        resurrect_min: 5,
        resurrect_max: 5,
    };
    for _ in 0..NUM_STEPS {
        game_of_life.step(&params);
    }
    Grid::new_grid_map(game_of_life.alive, |alive| {
        if alive {
            FloorOrWall::Floor
        } else {
            FloorOrWall::Wall
        }
    })
}

// Place walls at every point along the outside of a map
fn surround_map_with_walls(map: &mut Grid<FloorOrWall>) {
    let Coord {
        x: width,
        y: height,
    } = map.size().to_coord().unwrap();
    for (Coord { x, y }, cell) in map.enumerate_mut() {
        if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
            *cell = FloorOrWall::Wall;
        }
    }
}

// Remove clumps of wall cells which aren't connected to the edge of the map by walls (replacing
// them with floor cells)
fn remove_disconnected_walls(map: &mut Grid<FloorOrWall>) {
    assert!(
        *map.get_checked(Coord::new(0, 0)) == FloorOrWall::Wall,
        "top-left cell must be wall"
    );
    // Flood-fill all the wall cells starting with the top-left
    let mut walls_to_visit = vec![Coord::new(0, 0)];
    let mut seen = Grid::new_copy(map.size(), false);
    *seen.get_checked_mut(Coord::new(0, 0)) = true;
    while let Some(coord) = walls_to_visit.pop() {
        for neighbour_coord in CardinalDirection::all().map(|d| coord + d.coord()) {
            if let Some(FloorOrWall::Wall) = map.get(neighbour_coord) {
                let seen = seen.get_checked_mut(neighbour_coord);
                if !*seen {
                    *seen = true;
                    walls_to_visit.push(neighbour_coord);
                }
            }
        }
    }
    // Update the map, marking all unseen cells as floor
    for (cell_mut, &seen) in map.iter_mut().zip(seen.iter()) {
        if !seen {
            *cell_mut = FloorOrWall::Floor;
        }
    }
}

// Returns a grid of cells defining a cave map
fn generate_cave_map<R: Rng>(size: Size, rng: &mut R) -> Grid<FloorOrWall> {
    let mut map = generate_initial_cave_map(size, rng);
    surround_map_with_walls(&mut map);
    remove_disconnected_walls(&mut map);
    map
}

// A cell of the game world
#[derive(Clone, Copy, PartialEq, Eq)]
enum LevelCell {
    Floor,
    Wall,
    Door,
    CaveFloor,
    CaveWall,
}

impl LevelCell {
    fn is_wall(&self) -> bool {
        match self {
            Self::Wall | Self::CaveWall => true,
            _ => false,
        }
    }

    fn is_floor(&self) -> bool {
        match self {
            Self::Floor | Self::CaveFloor => true,
            _ => false,
        }
    }
}

// Returns true iff a given coordinate is entirely surrounded by walls
fn is_surrounded_by_walls(map: &Grid<RoomsAndCorridorsCell>, coord: Coord) -> bool {
    Direction::all()
        .filter_map(|direction| map.get(coord + direction.coord()))
        .all(|&cell| cell == RoomsAndCorridorsCell::Wall)
}

// Combines a map of rooms and corridors with a cave map to produce a hybrid of the two
fn combine_rooms_and_corridors_level_with_cave(
    rooms_and_corridors_level_map: &Grid<RoomsAndCorridorsCell>,
    cave_map: &Grid<FloorOrWall>,
) -> Grid<LevelCell> {
    Grid::new_fn(cave_map.size(), |coord| match cave_map.get_checked(coord) {
        FloorOrWall::Floor => LevelCell::CaveFloor,
        FloorOrWall::Wall => match rooms_and_corridors_level_map.get_checked(coord) {
            RoomsAndCorridorsCell::Floor => LevelCell::Floor,
            RoomsAndCorridorsCell::Door => LevelCell::Door,
            RoomsAndCorridorsCell::Wall => {
                if is_surrounded_by_walls(rooms_and_corridors_level_map, coord) {
                    LevelCell::CaveWall
                } else {
                    LevelCell::Wall
                }
            }
        },
    })
}

// Updates a map, replacing all cells unreachable from the player spawn with cave walls
fn remove_unreachable_floor(
    map: &mut Grid<LevelCell>,
    water_map: &mut Grid<bool>,
    player_spawn: Coord,
) {
    let mut seen = Grid::new_copy(map.size(), false);
    *seen.get_checked_mut(player_spawn) = true;
    let mut to_visit = vec![player_spawn];
    while let Some(current) = to_visit.pop() {
        for direction in CardinalDirection::all() {
            let neighbour_coord = current + direction.coord();
            if let Some(neighbour_cell) = map.get(neighbour_coord) {
                let water_cell = *water_map.get_checked(neighbour_coord);
                if !neighbour_cell.is_wall() || water_cell {
                    let seen_cell = seen.get_checked_mut(neighbour_coord);
                    if !*seen_cell {
                        to_visit.push(neighbour_coord);
                    }
                    *seen_cell = true;
                }
            }
        }
    }
    for ((&seen_cell, map_cell), water_cell) in
        seen.iter().zip(map.iter_mut()).zip(water_map.iter_mut())
    {
        if !seen_cell {
            *water_cell = false;
            if *map_cell == LevelCell::CaveFloor {
                *map_cell = LevelCell::CaveWall;
            }
        }
    }
}

// Returns true iff a given coordinate is a valid door position with respect to a given axis. That
// is, there is a floor cell on either side of the coordinate in the direction of the axis, and a
// wall cell on either side of the coordinate in the direction of the other axis.
fn is_valid_door_position_axis(map: &Grid<LevelCell>, coord: Coord, axis: Axis) -> bool {
    let axis_delta = Coord::new_axis(1, 0, axis);
    let other_axis_delta = Coord::new_axis(0, 1, axis);
    let floor_in_axis = map.get_checked(coord + axis_delta).is_floor()
        && map.get_checked(coord - axis_delta).is_floor();
    let wall_in_other_axis = map.get_checked(coord + other_axis_delta).is_wall()
        && map.get_checked(coord - other_axis_delta).is_wall();
    floor_in_axis && wall_in_other_axis
}

// Returns true iff a given coordinate is a valid door position
fn is_valid_door_position(map: &Grid<LevelCell>, coord: Coord) -> bool {
    is_valid_door_position_axis(map, coord, Axis::X)
        || is_valid_door_position_axis(map, coord, Axis::Y)
}

// Updates a map, replacing all door cells which aren't in valid positions with floor cells. A door
// can be in an invalid position due to the effect of combining a room and corridor map with a cave
// map.
fn remove_invalid_doors(map: &mut Grid<LevelCell>) {
    let to_remove = map
        .enumerate()
        .filter_map(|(coord, cell)| {
            if *cell == LevelCell::Door && !is_valid_door_position(map, coord) {
                Some(coord)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for coord in to_remove {
        *map.get_checked_mut(coord) = LevelCell::Floor;
    }
}

// Returns a grid of booleans, where a true value indicates that grass can spawn at that location.
// The grid is populated using perlin noise.
fn make_grass_map<R: Rng>(size: Size, rng: &mut R) -> Grid<bool> {
    let perlin = Perlin2::new(rng);
    let zoom = 10.;
    Grid::new_fn(size, |Coord { x, y }| {
        let x = x as f64 / zoom;
        let y = y as f64 / zoom;
        let noise = perlin.noise((x, y));
        noise > 0. && rng.gen::<f64>() > 0.5
    })
}

// Returns a grid of booleans, where a true value indicates that grass can spawn at that location.
// The grid is populated using perlin noise.
fn make_water_map<R: Rng>(size: Size, rng: &mut R) -> Grid<bool> {
    let perlin = Perlin2::new(rng);
    let zoom = 7.;
    let mut map = Grid::new_fn(size, |Coord { x, y }| {
        let x = x as f64 / zoom;
        let y = y as f64 / zoom;
        let noise = perlin.noise01((x, y));
        noise > 0.65
    });
    let mut to_visit = map
        .edge_enumerate()
        .filter_map(|(coord, cell)| if *cell { Some(coord) } else { None })
        .collect::<Vec<_>>();
    let mut seen = to_visit.iter().cloned().collect::<HashSet<_>>();
    while let Some(coord) = to_visit.pop() {
        for direction in CardinalDirection::all() {
            let neighbour_coord = coord + direction.coord();
            if let Some(true) = map.get(neighbour_coord) {
                if seen.insert(neighbour_coord) {
                    to_visit.push(neighbour_coord);
                }
            }
        }
    }
    for coord in seen {
        *map.get_checked_mut(coord) = false;
    }
    map
}

// Level representation produced by terrain generation
pub struct Terrain {
    pub world: World,
    pub player_entity: Entity,
}

impl Terrain {
    pub fn generate<R: Rng>(world_size: Size, rng: &mut R) -> Self {
        let mut world = World::new(world_size);
        let RoomsAndCorridorsLevel {
            map: rooms_and_corridors_map,
            player_spawn,
        } = RoomsAndCorridorsLevel::generate(world_size, rng);
        let cave_map = generate_cave_map(world_size, rng);
        let mut combined_map =
            combine_rooms_and_corridors_level_with_cave(&rooms_and_corridors_map, &cave_map);
        let mut water_map = make_water_map(world_size, rng);
        remove_unreachable_floor(&mut combined_map, &mut water_map, player_spawn);
        remove_invalid_doors(&mut combined_map);
        let grass_map = make_grass_map(world_size, rng);
        let player_entity = world.spawn_player(player_spawn);
        for (coord, &cell) in combined_map.enumerate() {
            use LevelCell::*;
            if *water_map.get_checked(coord) {
                match cell {
                    Floor | Door => world.spawn_water(coord),
                    Wall => {
                        if rng.gen_range(0..100) < 75 {
                            world.spawn_wall(coord)
                        } else {
                            world.spawn_water(coord);
                        }
                    }
                    CaveFloor | CaveWall => {
                        world.spawn_water(coord);
                        if *grass_map.get_checked(coord) {
                            world.spawn_grass(coord);
                        }
                    }
                }
            } else {
                match cell {
                    Floor => world.spawn_floor(coord),
                    Wall => world.spawn_wall(coord),
                    Door => world.spawn_door(coord),
                    CaveFloor => {
                        world.spawn_cave_floor(coord);
                        if *grass_map.get_checked(coord) {
                            world.spawn_grass(coord);
                        }
                    }
                    CaveWall => world.spawn_cave_wall(coord),
                }
            }
        }
        Self {
            world,
            player_entity,
        }
    }
}
