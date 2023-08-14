use std::time::Duration;

use bracket_terminal::prelude::*;
use draw_sprites::draw_sprite_layers;
use ldtk_map::prelude::*;
use mining::{DamageSystem, RemoveDeadTiles, TileDestructionSystem};
use monster::{check_monster_delay, RandomMonsterMovementSystem};
use specs::prelude::*;

mod draw_sprites;
mod indexing;
mod message_log;
mod mining;
mod monster;
mod player;
mod tile_animation;
mod user_interface;
use tile_animation::TileAnimationCleanUpSystem;
mod time;
use player::{check_player_activity, manage_player_input, PlayerResponse};
mod map;
use map::Map;
mod components;
use components::Position;
mod fishing;
use fishing::{CatchFishSystem, SetupFishingActions, WaitingForFishSystem};
use indexing::{IndexBlockedTiles, IndexBreakableTiles, IndexFishableTiles, IndexReset};
use tile_animation::TileAnimationSpawner;
use time::delta_time_update;
use user_interface::draw_ui;

use crate::{
    components::{
        Blocking, BreakAction, Breakable, DeleteCondition, FinishedActivity, FishAction,
        FishOnTheLine, Fishable, HealthStats, Monster, Name, RandomWalkerAI, Renderable, Strength,
        SufferDamage, WaitingForFish,
    },
    draw_sprites::debug_rocks,
    map::WorldTile,
    message_log::MessageLog,
    player::Player,
    tile_animation::TileAnimationBuilder,
    time::DeltaTime,
};

// Size of the terminal window
pub const DISPLAY_WIDTH: usize = 40;
pub const DISPLAY_HEIGHT: usize = 30;

// CL - Console layer, represents the indices for each console
pub const CL_TEXT: usize = 2; // Used for UI
pub const CL_WORLD: usize = 0; // Used for terrain tiles
pub const CL_INTERACTABLES: usize = 1; // Used for the few or so moving items/entities on screen

pub struct State {
    ecs: World,
}

impl State {
    fn run_response_systems(&mut self) {
        // println!("Response Systems are now running.");
        let mut randomwalker = RandomMonsterMovementSystem;
        randomwalker.run_now(&self.ecs);
        // println!("Response Systems are now finished.");
    }

    fn run_continuous_systems(&mut self, _ctx: &mut BTerm) {
        // println!("Continuous Systems are now running.");
        // Indexing systems
        let mut indexreset = IndexReset;
        indexreset.run_now(&self.ecs);
        let mut indexblocking = IndexBlockedTiles;
        indexblocking.run_now(&self.ecs);
        let mut indexbreaking = IndexBreakableTiles;
        indexbreaking.run_now(&self.ecs);
        let mut indexfishing = IndexFishableTiles;
        indexfishing.run_now(&self.ecs);

        let mut setupfishingactions = SetupFishingActions;
        setupfishingactions.run_now(&self.ecs);
        let mut waitingforfishsystem = WaitingForFishSystem;
        waitingforfishsystem.run_now(&self.ecs);
        let mut catchfishsystem = CatchFishSystem;
        catchfishsystem.run_now(&self.ecs);

        let mut mining_sys = TileDestructionSystem;
        mining_sys.run_now(&self.ecs);
        let mut damage_sys = DamageSystem;
        damage_sys.run_now(&self.ecs);

        // Request based system run as late as possible in the loop
        let mut tile_anim_spawner = TileAnimationSpawner { world: &self.ecs };
        tile_anim_spawner.run_now(&self.ecs);

        let mut tile_anim_cleanup_system = TileAnimationCleanUpSystem;
        tile_anim_cleanup_system.run_now(&self.ecs);

        let mut remove_dead_tiles = RemoveDeadTiles;
        remove_dead_tiles.run_now(&self.ecs);

        // println!("Continuous Systems are now finished.");
    }

    /// Systems that need to be ran after most other systems are finished EOF - end of frame
    fn run_eof_systems(&mut self) {
        self.ecs.write_storage::<FinishedActivity>().clear();
    }
}

/// Defines the app's state for the game
#[derive(Clone, Copy)]
pub enum AppState {
    InMenu,
    InGame,
    ActivityBound { response_delay: Duration }, // can only perform a specific acitivity that is currently happening
}

impl AppState {
    /// Creates the enum variant ActivityBound with zero duration
    pub fn activity_bound() -> Self {
        Self::ActivityBound {
            response_delay: Duration::ZERO,
        }
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        let mut new_state: AppState;
        {
            // this is in a new scope because we need to mutate self (the ecs) later in the fn
            let current_state = self.ecs.fetch::<AppState>();
            new_state = *current_state;
        }

        match new_state {
            AppState::InMenu => {
                todo!("player input will control the menu, when menus are implemented")
            }
            AppState::InGame => {
                // if we have to run something before player put it here >>>
                match manage_player_input(self, ctx) {
                    PlayerResponse::Waiting => {
                        // Player hasn't done anything yet so only run essential systems
                    }
                    PlayerResponse::TurnAdvance => {
                        self.run_response_systems();
                    }
                    PlayerResponse::StateChange(delta_state) => {
                        new_state = delta_state;
                    }
                }
                self.run_continuous_systems(ctx);
                self.run_eof_systems();
                delta_time_update(&mut self.ecs, ctx);
            }
            AppState::ActivityBound { mut response_delay } => {
                // if the player finishes we run final systems and change state
                self.run_continuous_systems(ctx);
                new_state = if check_player_activity(&mut self.ecs) {
                    AppState::InGame
                } else if check_monster_delay(&self.ecs, &mut response_delay) {
                    // if the monster delay timer is past its due then monsters do their thing
                    self.run_response_systems();
                    AppState::activity_bound()
                } else {
                    AppState::ActivityBound { response_delay }
                };

                self.run_eof_systems();
                delta_time_update(&mut self.ecs, ctx);
            }
        }

        self.ecs.maintain();
        draw_ui(&self.ecs, ctx);
        draw_sprite_layers(&self.ecs, ctx);

        // Insert the state resource to overwrite it's existing and update the state of the app
        let mut state_writer = self.ecs.write_resource::<AppState>();
        *state_writer = new_state;
    }
}

bracket_terminal::embedded_resource!(TILE_FONT, "../resources/interactable_tiles.png");
bracket_terminal::embedded_resource!(CHAR_FONT, "../resources/terminal8x8.png");
bracket_terminal::embedded_resource!(TERRAIN_FOREST, "../resources/terrain_forest.png");

fn main() -> BError {
    bracket_terminal::link_resource!(TILE_FONT, "resources/interactable_tiles.png");
    bracket_terminal::link_resource!(CHAR_FONT, "resources/terminal8x8.png");
    bracket_terminal::link_resource!(TERRAIN_FOREST, "resources/terrain_forest.png");

    // Setup Terminal (incl Window, Input, Font Loading)
    let context = BTermBuilder::new()
        .with_title("Tile RPG")
        .with_fps_cap(60.0)
        .with_font("terminal8x8.png", 8u32, 8u32)
        .with_font("interactable_tiles.png", 8u32, 8u32)
        .with_font("terrain_forest.png", 8u32, 8u32)
        .with_dimensions(DISPLAY_WIDTH * 2, DISPLAY_HEIGHT * 2)
        .with_simple_console(DISPLAY_WIDTH, DISPLAY_HEIGHT, "terrain_forest.png")
        .with_fancy_console(DISPLAY_WIDTH, DISPLAY_HEIGHT, "interactable_tiles.png")
        .with_sparse_console(DISPLAY_WIDTH * 2, DISPLAY_HEIGHT * 2, "terminal8x8.png")
        .build()?;

    register_palette_color("pink", RGB::named(MAGENTA));

    // Setup ECS
    let mut world = World::new();
    // Component Registration, the ECS needs to have every type of component registered
    world.register::<Position>();
    world.register::<Player>();
    world.register::<Renderable>();
    world.register::<Blocking>();
    world.register::<HealthStats>();
    world.register::<BreakAction>();
    world.register::<Breakable>();
    world.register::<SufferDamage>();
    world.register::<Strength>();
    world.register::<Fishable>();
    world.register::<FishAction>();
    world.register::<WaitingForFish>();
    world.register::<FishOnTheLine>();
    world.register::<DeleteCondition>();
    world.register::<FinishedActivity>();
    world.register::<Name>();
    world.register::<Monster>();
    world.register::<RandomWalkerAI>();

    // Resource Initialization, the ECS needs a basic definition of every resource that will be in the game
    world.insert(DeltaTime(Duration::ZERO));
    world.insert(TileAnimationBuilder::new());
    world.insert(AppState::InGame);
    world.insert(MessageLog::new());

    // A very plain map
    let mut map = Map::new(DISPLAY_WIDTH, DISPLAY_HEIGHT - 3);
    let water_idx = map.xy_to_idx(10, 15);
    map.tiles[water_idx] = WorldTile { atlas_index: 80 };
    world
        .create_entity()
        .with(Position::new(10, 15))
        .with(Fishable)
        .with(Blocking)
        .build();

    world.insert(map);

    world
        .create_entity()
        .with(Position::new(17, 20))
        .with(Player)
        .with(Strength { amt: 1 })
        .with(Renderable::new(ColorPair::new(WHITE, BLACK), 2))
        .with(Blocking)
        .build();

    world
        .create_entity()
        .with(Position::new(5, 15))
        .with(Monster)
        .with(Name::new("Bahhhby"))
        .with(RandomWalkerAI)
        .with(Renderable::new(ColorPair::new(WHITE, BLACK), 16))
        .with(Blocking)
        .build();

    debug_rocks(&mut world);

    let game_state: State = State { ecs: world };
    main_loop(context, game_state)
}
