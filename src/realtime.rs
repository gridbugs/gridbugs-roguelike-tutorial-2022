use crate::game::World;
use gridbugs::{
    entity_table::Entities,
    entity_table_realtime::{declare_realtime_entity_module, ContextContainsRealtimeComponents},
};
use rand_isaac::Isaac64Rng;

pub struct RealtimeContext<'a> {
    pub world: &'a mut World,
    pub rng: &'a mut Isaac64Rng,
}

impl<'a> ContextContainsRealtimeComponents for RealtimeContext<'a> {
    type Components = RealtimeComponents;
    fn components_mut(&mut self) -> &mut Self::Components {
        self.world.realtime_components_mut()
    }
    fn realtime_entities(&self) -> Entities {
        self.world.realtime_entities()
    }
}

mod water_animation {
    use super::RealtimeContext;
    use crate::game::ColourHint;
    use gridbugs::{
        entity_table::Entity,
        entity_table_realtime::{RealtimeComponent, RealtimeComponentApplyEvent},
        rgb_int::Rgb24,
    };
    use rand::{Rng, SeedableRng};
    use rand_isaac::Isaac64Rng;
    use std::time::Duration;

    #[derive(Clone, Debug)]
    pub struct State {
        rng: Isaac64Rng,
    }

    impl State {
        pub fn new<R: Rng>(rng: &mut R) -> Self {
            Self {
                rng: Isaac64Rng::from_rng(rng).unwrap(),
            }
        }
    }

    pub struct UpdateColourHint;

    impl RealtimeComponent for State {
        type Event = UpdateColourHint;

        fn tick(&mut self) -> (Self::Event, std::time::Duration) {
            let until_next_tick_millis = self.rng.gen_range(100..1000);
            (
                UpdateColourHint,
                Duration::from_millis(until_next_tick_millis),
            )
        }
    }

    const WATER_COLOUR_DIM: Rgb24 = Rgb24::new(0, 31, 63);
    const WATER_COLOUR_BRIGHT: Rgb24 = Rgb24::new(0, 63, 127);

    impl<'a> RealtimeComponentApplyEvent<RealtimeContext<'a>> for State {
        fn apply_event(_: UpdateColourHint, entity: Entity, context: &mut RealtimeContext<'a>) {
            let colour_hint_component = &mut context.world.components_mut().colour_hint;
            let mut choose_colour = || context.rng.gen_range(WATER_COLOUR_DIM..WATER_COLOUR_BRIGHT);
            if let Some(&(mut colour_hint)) = colour_hint_component.get(entity) {
                let new_colour = choose_colour();
                if context.rng.gen() {
                    colour_hint.foreground = new_colour;
                } else {
                    colour_hint.background = new_colour;
                }
                colour_hint_component.insert(entity, colour_hint);
            } else {
                colour_hint_component.insert(
                    entity,
                    ColourHint {
                        foreground: choose_colour(),
                        background: choose_colour(),
                    },
                );
            }
        }
    }
}

pub mod types {
    pub use super::water_animation::State as WaterAnimationState;
}

declare_realtime_entity_module! {
    components<'a>[RealtimeContext<'a>] {
        water_animation: types::WaterAnimationState,
    }
}

pub use components::RealtimeComponents;
