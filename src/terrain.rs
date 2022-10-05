use crate::game::World;
use gridbugs::{
    coord_2d::{Coord, Size},
    entity_table::Entity,
};
use rand::Rng;

// Level representation produced by terrain generation
pub struct Terrain {
    pub world: World,
    pub player_entity: Entity,
}

impl Terrain {
    pub fn generate<R: Rng>(world_size: Size, rng: &mut R) -> Self {
        let mut world = World::new(world_size);
        let centre = world_size.to_coord().unwrap() / 2;
        // The player starts in the centre of the screen
        let player_entity = world.spawn_player(Coord {
            x: rng.gen_range(0..world_size.width() as i32),
            y: rng.gen_range(0..world_size.height() as i32),
        });
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
