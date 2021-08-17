use euclid::{vec2, Vector2D};
use hecs::{Entity, EntityBuilder, World};
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

struct Target {
    pos: Vector,
    speed: f32,
}

impl Target {
    fn new(pos: Vector) -> Self {
        Self { pos, speed: 2.0 }
    }
}

struct Animation {
    dir: Angle,
    frame: f32,
}

impl Animation {
    fn new() -> Self {
        Self {
            dir: Angle(0),
            frame: 0.0,
        }
    }
}

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
struct Controls {
    up: KeyCode,
    down: KeyCode,
    left: KeyCode,
    right: KeyCode,
    shoot: KeyCode,
}

const team_controls: [Controls; 2] = [
    Controls {
        up: KeyCode::Up,
        down: KeyCode::Down,
        left: KeyCode::Left,
        right: KeyCode::Right,
        shoot: KeyCode::Space,
    },
    Controls {
        up: KeyCode::W,
        down: KeyCode::S,
        left: KeyCode::A,
        right: KeyCode::D,
        shoot: KeyCode::LeftShift,
    },
];

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
    fn sin(&self) -> f32 {
        (self.0 as f32 * PI / 4.0).sin()
    }

    fn cos(&self) -> f32 {
        (self.0 as f32 * PI / 4.0).cos()
    }

    fn from_vec(v: Vector) -> Self {
        Angle((((4.0 / PI * v.x.atan2(-v.y)) + 8.5) as i32) % 8)
    }

    fn to_vec(a: Self) -> Vector {
        vec2(a.sin(), -a.cos())
    }
}

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

struct TeamInfo {
    controls: Option<Controls>,
    score: u8,
    active_player: Option<Entity>,
}

impl TeamInfo {
    fn new(controls: Option<Controls>) -> Self {
        Self {
            controls,
            score: 0,
            active_player: None,
        }
    }

    fn human(&self) -> bool {
        self.controls.is_some()
    }
}

struct Game {
    difficulty: Difficulty,
    camera_focus: Vector,
    world: World,
    ball: Entity,
    teams: [TeamInfo; 2],
}

impl Game {
    fn new(difficulty: Difficulty) -> Self {
        let mut world = World::new();
        let mut eb = EntityBuilder::new();
        build_ball(&mut eb);
        let ball = world.spawn(eb.build());
        let mut me = Self {
            difficulty,
            camera_focus: vec2(HALF_LEVEL_W as f32, HALF_LEVEL_H as f32),
            world,
            ball,
            teams: [TeamInfo::new(None), TeamInfo::new(None)],
        };
        me.add_players();
        me
    }

    fn reset(&mut self) {
        self.world.clear();
        let mut eb = EntityBuilder::new();
        build_ball(&mut eb);
        self.ball = self.world.spawn(eb.build());
        self.add_players();
    }

    fn add_players(&mut self) {
        let mut ids = Vec::new();
        let mut eb = EntityBuilder::new();
        for (x, y) in PLAYER_START_POS {
            build_player(&mut eb, x, y, 550., 0);
            ids.push(self.world.spawn(eb.build()));
            build_player(&mut eb, LEVEL_W - x, LEVEL_H - y, 150., 1);
            ids.push(self.world.spawn(eb.build()));
        }
        self.teams[0].active_player = Some(ids[0]);
        self.teams[1].active_player = Some(ids[1]);
    }

    fn update(&mut self) {
        // todo check for goal scored
        // todo set behaviours (mark, leads, goalie)
        self.set_player_targets();
        update_players(&mut self.world, self.ball);
        // todo update ball
        // todo handle team switching
        // todo update camera
    }

    fn set_player_targets(&mut self) {
        // todo: everything here
    }
}

fn build_ball(eb: &mut EntityBuilder) {
    eb.add(Position(vec2(HALF_LEVEL_W as f32, HALF_LEVEL_H as f32)));
    eb.add(Ball);
}

fn build_player(eb: &mut EntityBuilder, x: f32, y: f32, offs: f32, team: u8) {
    let x = x + gen_range(-32., 32.);
    let y = y + gen_range(-32., 32.);
    eb.add(Home(vec2(x, y)));
    let start = vec2(x, y / 2. + offs);
    eb.add(Position(start.clone()));
    //eb.add(Target(start));
    eb.add(Target::new(vec2(500., 700.)));
    eb.add(Team(team));
    eb.add(Animation::new());
}

fn update_players(world: &mut World, ball: Entity) {
    let ball_pos = world.get::<Position>(ball).unwrap();
    for (_, (target, pos, anim)) in &mut world.query::<(&Target, &mut Position, &mut Animation)>() {
        let vector = target.pos - pos.0;
        let target_dir;
        let length = vector.length();
        if length == 0.0 {
            target_dir = Angle::from_vec(ball_pos.0 - pos.0);
            anim.frame = 0.0;
        } else {
            let vector = vector.with_max_length(target.speed);
            target_dir = Angle::from_vec(vector);
            // todo: should do the allow_movement thing to slide here
            // instead do this
            pos.0 += vector;
            anim.frame += vector.length().max(3.0); // todo tweak this
            anim.frame %= 72.0;
        }
        // update facing direction here
        // todo: should be gradual
        anim.dir = target_dir;
    }
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
    textures.preload("arrow0").await;
    textures.preload("arrow1").await;
    for d in 0..=7 {
        for f in 0..=4 {
            textures.preload(format!("player0{}{}", d, f)).await;
            textures.preload(format!("player1{}{}", d, f)).await;
            textures.preload(format!("players{}{}", d, f)).await;
        }
    }
    for k in vec!["01", "02", "10", "11", "12"] {
        textures.preload(format!("menu{}", k)).await;
    }
    // todo set up sound
    let mut state = State::Menu(MenuState::NumPlayers, Settings::new());
    let mut game = Game::new(get_difficulty(DifficultyLevel::Hard));
    loop {
        match state {
            State::Menu(ref mut menu_state, ref mut settings) => {
                if is_key_pressed(KeyCode::Space) {
                    match menu_state {
                        MenuState::Difficulty => {
                            game = Game::new(get_difficulty(settings.difficulty_level));
                            game.teams[0].controls = Some(team_controls[0]);
                            game.teams[1].controls = None;
                            state = State::Play;
                        }
                        MenuState::NumPlayers => match settings.num_players {
                            NumPlayers::One => {
                                *menu_state = MenuState::Difficulty;
                            }
                            NumPlayers::Two => {
                                game = Game::new(get_difficulty(DifficultyLevel::Hard));
                                game.teams[0].controls = Some(team_controls[0]);
                                game.teams[1].controls = Some(team_controls[1]);
                                state = State::Play;
                            }
                        },
                    };
                } else {
                    let mut change = MenuChange::NoChange;
                    if is_key_pressed(KeyCode::Up) {
                        change = MenuChange::Up;
                    } else if is_key_pressed(KeyCode::Down) {
                        change = MenuChange::Down;
                    }
                    if change != MenuChange::NoChange {
                        // todo play "move" sound
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

        game.update();

        let offs_x = (game.camera_focus.x - WIDTH as f32 / 2.)
            .min(LEVEL_W - WIDTH)
            .max(0.0) as f32;
        let offs_y = (game.camera_focus.y - HEIGHT as f32 / 2.)
            .min(LEVEL_H - HEIGHT)
            .max(0.0) as f32;
        draw_texture(textures.get("pitch"), -offs_x, -offs_y, WHITE);

        let mut sprites: Vec<(String, f32, f32)> = Vec::new();

        for (_id, (pos, team, anim)) in &mut game.world.query::<(&Position, &Team, &Animation)>() {
            let suffix = format!("{}{}", anim.dir.0, (anim.frame as u32 / 18));
            sprites.push((
                format!("player{}{}", team.0, suffix).to_owned(),
                pos.0.x - offs_x - 25., // hardcoded anchor
                pos.0.y - offs_y - 37., // hardcoded anchor
            ));
            draw_texture(
                textures.get(&format!("players{}", suffix)),
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

        for t in 0..=1 {
            if game.teams[t].human() {
                if let Some(id) = game.teams[t].active_player {
                    if let Ok(pos) = game.world.get::<Position>(id) {
                        draw_texture(
                            textures.get(&format!("arrow{}", t)),
                            pos.0.x - offs_x - 11.,
                            pos.0.y - offs_y - 45.,
                            WHITE,
                        )
                    }
                }
            }
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
                // todo display score bar at top
                // todo show score for each team
                // todo show GOAL if a goal has recently been scored
            }
            State::GameOver => {
                // todo display GAME OVER image
                // todo show score for each team
            }
        }
        next_frame().await;
    }
}
