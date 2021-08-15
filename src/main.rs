use euclid::{vec2, Vector2D};
use hecs::{DynamicBundle, Entity, EntityBuilder, World};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::collections::HashMap;
use std::f32::consts::PI;

enum PixelUnit {}

type Vector = Vector2D<f32, PixelUnit>;

#[derive(Debug)]
struct Position(Vector);
struct Home(Vector);
struct Team(u8);
struct Ball();

const HEIGHT: f32 = 480.0;
const WIDTH: f32 = 800.0;

const HALF_WINDOW_WIDTH: f32 = WIDTH / 2.0;

const LEVEL_W: f32 = 1000.0;
const LEVEL_H: f32 = 1400.0;
const HALF_LEVEL_W: f32 = LEVEL_W / 2.0;
const HALF_LEVEL_H: f32 = LEVEL_H / 2.0;

const HALF_PITCH_W: f32 = 442.0;
const HALF_PITCH_H: f32 = 622.0;

const GOAL_WIDTH: f32 = 186.0;
const GOAL_DEPTH: f32 = 20.0;
const HALF_GOAL_W: f32 = GOAL_WIDTH / 2.0;

/*
PITCH_BOUNDS_X = (HALF_LEVEL_W - HALF_PITCH_W, HALF_LEVEL_W + HALF_PITCH_W)
PITCH_BOUNDS_Y = (HALF_LEVEL_H - HALF_PITCH_H, HALF_LEVEL_H + HALF_PITCH_H)

GOAL_BOUNDS_X = (HALF_LEVEL_W - HALF_GOAL_W, HALF_LEVEL_W + HALF_GOAL_W)
GOAL_BOUNDS_Y = (HALF_LEVEL_H - HALF_PITCH_H - GOAL_DEPTH,
                 HALF_LEVEL_H + HALF_PITCH_H + GOAL_DEPTH)

PITCH_RECT = pygame.rect.Rect(PITCH_BOUNDS_X[0], PITCH_BOUNDS_Y[0], HALF_PITCH_W * 2, HALF_PITCH_H * 2)
GOAL_0_RECT = pygame.rect.Rect(GOAL_BOUNDS_X[0], GOAL_BOUNDS_Y[0], GOAL_WIDTH, GOAL_DEPTH)
GOAL_1_RECT = pygame.rect.Rect(GOAL_BOUNDS_X[0], GOAL_BOUNDS_Y[1] - GOAL_DEPTH, GOAL_WIDTH, GOAL_DEPTH)

AI_MIN_X = 78
AI_MAX_X = LEVEL_W - 78
AI_MIN_Y = 98
AI_MAX_Y = LEVEL_H - 98
*/

const PLAYER_START_POS: [(f32, f32); 7] = [
    (350., 550.),
    (650., 450.),
    (200., 850.),
    (500., 750.),
    (800., 950.),
    (350., 1250.),
    (650., 1150.),
];

/*
LEAD_DISTANCE_1 = 10
LEAD_DISTANCE_2 = 50

DRIBBLE_DIST_X, DRIBBLE_DIST_Y = 18, 16

# Speeds for players in various situations. Speeds including 'BASE' can be boosted by the speed_boost difficulty
# setting (only for players on a computer-controlled team)
PLAYER_DEFAULT_SPEED = 2
CPU_PLAYER_WITH_BALL_BASE_SPEED = 2.6
PLAYER_INTERCEPT_BALL_SPEED = 2.75
LEAD_PLAYER_BASE_SPEED = 2.9
HUMAN_PLAYER_WITH_BALL_SPEED = 3
HUMAN_PLAYER_WITHOUT_BALL_SPEED = 3.3

DEBUG_SHOW_LEADS = False
DEBUG_SHOW_TARGETS = False
DEBUG_SHOW_PEERS = False
DEBUG_SHOW_SHOOT_TARGET = False
DEBUG_SHOW_COSTS = False
*/

#[derive(Copy, Clone)]
enum DifficultyLevel {
    Easy = 0,
    Medium = 1,
    Hard = 2,
}

struct Difficulty {
    goalie_enabled: bool,
    second_lead_enabled: bool,
    speed_boost: f32,
    holdoff_timer: i32,
}

fn get_difficulty(level: DifficultyLevel) -> Difficulty {
    match level {
        DifficultyLevel::Easy => Difficulty {
            goalie_enabled: false,
            second_lead_enabled: false,
            speed_boost: 0.0,
            holdoff_timer: 120,
        },
        DifficultyLevel::Medium => Difficulty {
            goalie_enabled: false,
            second_lead_enabled: true,
            speed_boost: 0.1,
            holdoff_timer: 90,
        },
        DifficultyLevel::Hard => Difficulty {
            goalie_enabled: true,
            second_lead_enabled: true,
            speed_boost: 0.2,
            holdoff_timer: 60,
        },
    }
}

struct Angle(i32);

impl Angle {
    fn sin(a: Self) -> f32 {
        (a.0 as f32 * PI / 4.0).sin()
    }

    fn cos(a: Self) -> f32 {
        (a.0 as f32 * PI / 4.0).cos()
    }
}

// vec_to_angle
// angle_to_vec

// all the game logic ...

enum State {
    Menu(MenuState, Settings),
    Play,
    GameOver,
}

enum MenuState {
    NumPlayers,
    Difficulty,
}

#[derive(Copy, Clone)]
enum NumPlayers {
    One = 1,
    Two = 2,
}

struct Settings {
    num_players: NumPlayers,
    difficulty_level: DifficultyLevel,
}

impl Settings {
    fn new() -> Self {
        Self {
            num_players: NumPlayers::One,
            difficulty_level: DifficultyLevel::Medium,
        }
    }
}

#[derive(PartialEq)]
enum MenuChange {
    Up,
    Down,
    NoChange,
}

struct Game {
    difficulty: Difficulty,
    camera_focus: Vector,
    world: World,
    ball: Entity,
}

impl Game {
    fn new(difficulty: Difficulty) -> Self {
        let mut world = World::new();
        let ball = world.spawn(make_ball().build());
        let mut me = Self {
            difficulty,
            camera_focus: vec2(HALF_LEVEL_W as f32, HALF_LEVEL_H as f32),
            world,
            ball,
        };
        me.add_players();
        me
    }

    fn reset(&mut self) {
        self.world.clear();
        self.ball = self.world.spawn(make_ball().build());
        self.add_players();
    }

    fn add_players(&mut self) {
        let mut eb = EntityBuilder::new();
        for (x, y) in PLAYER_START_POS {
            {
                let x = x + gen_range(-32., 32.);
                let y = y + gen_range(-32., 32.);
                eb.add(Home(vec2(x, y)));
                eb.add(Position(vec2(x, y / 2. + 550.)));
                eb.add(Team(0));
                self.world.spawn(eb.build());
            }
            {
                let x = LEVEL_W - x + gen_range(-32., 32.);
                let y = LEVEL_H - y + gen_range(-32., 32.);
                eb.add(Home(vec2(x, y)));
                eb.add(Position(vec2(x, y / 2. + 150.)));
                eb.add(Team(1));
                self.world.spawn(eb.build());
            }
        }
    }
}

fn make_ball() -> EntityBuilder {
    let mut eb = EntityBuilder::new();
    eb.add(Position(vec2(HALF_LEVEL_W as f32, HALF_LEVEL_H as f32)));
    eb.add(Ball);
    eb
}

fn window_conf() -> Conf {
    return Conf {
        window_title: "Substitute Soccer".to_owned(),
        window_width: WIDTH as i32,
        window_height: HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    };
}

struct Textures(HashMap<String, Texture2D>);

impl Textures {
    fn new() -> Self {
        return Self(HashMap::new());
    }
    async fn preload(&mut self, key: impl Into<String>) {
        let key: String = key.into();
        let texture = load_texture(&format!("images/{}.png", key)).await.unwrap();
        println!("Loaded texture: {}", key);
        self.0.insert(key, texture);
    }
    fn get(&self, key: &str) -> Texture2D {
        *self.0.get(key).unwrap()
    }
}

#[macroquad::main(window_conf())]
async fn main() {
    macroquad::rand::srand(macroquad::miniquad::date::now() as u64);
    // load all the textures
    let mut textures = Textures::new();
    textures.preload("pitch").await;
    textures.preload("ball").await;
    textures.preload("balls").await;
    for k in vec!["01", "02", "10", "11", "12"] {
        textures.preload(format!("menu{}", k)).await;
    }
    for k in vec!["000", "001", "100", "101"] {
        textures.preload(format!("player{}", k)).await;
    }
    for k in vec!["00", "01"] {
        textures.preload(format!("players{}", k)).await;
    }
    // set up sound
    let mut state = State::Menu(MenuState::NumPlayers, Settings::new());
    let mut game = Game::new(get_difficulty(DifficultyLevel::Hard));
    loop {
        match state {
            State::Menu(ref mut menu_state, ref mut settings) => {
                if is_key_pressed(KeyCode::Space) {
                    match menu_state {
                        MenuState::Difficulty => {
                            /* start game */
                            state = State::Play;
                        }
                        MenuState::NumPlayers => {
                            match settings.num_players {
                                NumPlayers::One => {
                                    *menu_state = MenuState::Difficulty;
                                }
                                NumPlayers::Two => {
                                    /* start game */
                                    state = State::Play;
                                }
                            }
                        }
                    };
                } else {
                    let mut change = MenuChange::NoChange;
                    if is_key_pressed(KeyCode::Up) {
                        change = MenuChange::Up;
                    } else if is_key_pressed(KeyCode::Down) {
                        change = MenuChange::Down;
                    }
                    if change != MenuChange::NoChange {
                        // play "move" sound
                        match menu_state {
                            MenuState::NumPlayers => {
                                settings.num_players = match settings.num_players {
                                    NumPlayers::One => NumPlayers::Two,
                                    NumPlayers::Two => NumPlayers::One,
                                }
                            }
                            MenuState::Difficulty => {
                                settings.difficulty_level =
                                    match (settings.difficulty_level, change) {
                                        (DifficultyLevel::Easy, MenuChange::Up) => {
                                            DifficultyLevel::Hard
                                        }
                                        (DifficultyLevel::Easy, MenuChange::Down) => {
                                            DifficultyLevel::Medium
                                        }
                                        (DifficultyLevel::Medium, MenuChange::Up) => {
                                            DifficultyLevel::Easy
                                        }
                                        (DifficultyLevel::Medium, MenuChange::Down) => {
                                            DifficultyLevel::Hard
                                        }
                                        (DifficultyLevel::Hard, MenuChange::Up) => {
                                            DifficultyLevel::Medium
                                        }
                                        (DifficultyLevel::Hard, MenuChange::Down) => {
                                            DifficultyLevel::Easy
                                        }
                                        (_, MenuChange::NoChange) => unreachable!(),
                                    }
                            }
                        }
                    }
                }
            }
            State::Play => (),
            State::GameOver => (),
        }

        let offs_x = (game.camera_focus.x - WIDTH as f32 / 2.)
            .min(LEVEL_W - WIDTH)
            .max(0.0) as f32;
        let offs_y = (game.camera_focus.y - HEIGHT as f32 / 2.)
            .min(LEVEL_H - HEIGHT)
            .max(0.0) as f32;
        draw_texture(textures.get("pitch"), -offs_x, -offs_y, WHITE);

        let mut sprites: Vec<(String, f32, f32)> = Vec::new();

        for (_id, (pos, team)) in &mut game.world.query::<(&Position, &Team)>() {
            sprites.push((
                format!("player{}00", team.0).to_owned(),
                pos.0.x - offs_x - 25., // hardcoded anchor
                pos.0.y - offs_y - 37., // hardcoded anchor
            ));
            draw_texture(
                textures.get("players00"),
                pos.0.x - offs_x - 25.,
                pos.0.y - offs_y - 37.,
                WHITE,
            );
        }

        let ball_pos = &*game.world.get::<Position>(game.ball).unwrap();
        sprites.push((
            "ball".to_owned(),
            ball_pos.0.x - offs_x - 12.5,
            ball_pos.0.y - offs_y - 12.5,
        ));
        draw_texture(
            textures.get("balls"),
            ball_pos.0.x - offs_x - 12.5,
            ball_pos.0.y - offs_y - 12.5,
            WHITE,
        );

        sprites.sort_unstable_by(|(_, _, y1), (_, _, y2)| {
            y1.partial_cmp(y2).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (key, x, y) in sprites {
            draw_texture(textures.get(&key), x, y, WHITE);
        }

        match state {
            State::Menu(ref menu_state, ref settings) => {
                let key = match menu_state {
                    MenuState::NumPlayers => {
                        format!("menu0{}", settings.num_players as usize).to_owned()
                    }
                    MenuState::Difficulty => {
                        format!("menu1{}", settings.difficulty_level as usize).to_owned()
                    }
                };
                draw_texture(textures.get(&key), 0.0, 0.0, WHITE);
            }
            State::Play => {
                // display score bar at top
                // show score for each team
                // show GOAL if a goal has recently been scored
            }
            State::GameOver => {
                // display GAME OVER image
                // show score for each team
            }
        }
        next_frame().await;
    }
}
