use macroquad::prelude::*;
use std::collections::HashMap;
use std::f32::consts::PI;

const HEIGHT: i32 = 480;
const WIDTH: i32 = 800;

const HALF_WINDOW_WIDTH: i32 = WIDTH / 2;

const LEVEL_W: i32 = 1000;
const LEVEL_H: i32 = 1400;
const HALF_LEVEL_W: i32 = LEVEL_W / 2;
const HALF_LEVEL_H: i32 = LEVEL_H / 2;

const HALF_PITCH_W: i32 = 442;
const HALF_PITCH_H: i32 = 622;

const GOAL_WIDTH: i32 = 186;
const GOAL_DEPTH: i32 = 20;
const HALF_GOAL_W: i32 = GOAL_WIDTH / 2;

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

PLAYER_START_POS = [(350, 550), (650, 450), (200, 850), (500, 750), (800, 950), (350, 1250), (650, 1150)]

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

/*
struct Game {
    num_players: NumPlayers,
    difficulty_level: DifficultyLevel,
}
*/

fn window_conf() -> Conf {
    return Conf {
        window_title: "Substitute Soccer".to_owned(),
        window_width: WIDTH,
        window_height: HEIGHT,
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
    // load all the textures
    let mut textures = Textures::new();
    textures.preload("pitch").await;
    for k in vec!["01", "02", "10", "11", "12"] {
        textures.preload(format!("menu{}", k)).await;
    }
    for k in vec!["000", "001", "002", "003", "004"] {
        textures.preload(format!("player{}", k)).await;
    }
    // set up sound
    let mut state = State::Menu(MenuState::NumPlayers, Settings::new());
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

        draw_texture(textures.get("pitch"), 0.0, 0.0, WHITE);

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
            State::Play => (),
            State::GameOver => (),
        }
        next_frame().await;
    }
}
