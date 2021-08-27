use euclid::{vec2, Vector2D};
use hecs::{Entity, EntityBuilder, World};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::collections::HashMap;
use std::f32::consts::PI;

enum PixelUnit {}

type Vector = Vector2D<f32, PixelUnit>;

#[derive(Debug, Clone, Copy)]
struct Position(Vector);
struct Home(Vector);
struct Team(u8);
struct Ball();
struct Peer(Entity);

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

struct Timer(i8);

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

const PITCH_BOUNDS_X: (f32, f32) = (HALF_LEVEL_W - HALF_PITCH_W, HALF_LEVEL_W + HALF_PITCH_W);
const PITCH_BOUNDS_Y: (f32, f32) = (HALF_LEVEL_H - HALF_PITCH_H, HALF_LEVEL_H + HALF_PITCH_H);

const GOAL_BOUNDS_X: (f32, f32) = (HALF_LEVEL_W - HALF_GOAL_W, HALF_LEVEL_W + HALF_GOAL_W);
const GOAL_BOUNDS_Y: (f32, f32) = (
    HALF_LEVEL_H - HALF_PITCH_H - GOAL_DEPTH,
    HALF_LEVEL_H + HALF_PITCH_H + GOAL_DEPTH,
);

fn on_pitch(x: f32, y: f32) -> bool {
    (x >= PITCH_BOUNDS_X.0 && x < PITCH_BOUNDS_X.1 && y >= PITCH_BOUNDS_Y.0 && y < PITCH_BOUNDS_Y.1)
        || (x >= GOAL_BOUNDS_X.0
            && x < GOAL_BOUNDS_X.1
            && y >= GOAL_BOUNDS_Y.0
            && y < GOAL_BOUNDS_Y.1)
}

const AI_MIN_X: f32 = 78.0;
const AI_MAX_X: f32 = LEVEL_W - 78.0;
const AI_MIN_Y: f32 = 98.0;
const AI_MAX_Y: f32 = LEVEL_H - 98.0;

const KICK_STRENGTH: f32 = 11.5;
const DRAG: f32 = 0.98;

const PLAYER_START_POS: [(f32, f32); 7] = [
    (350., 550.),
    (650., 450.),
    (200., 850.),
    (500., 750.),
    (800., 950.),
    (350., 1250.),
    (650., 1150.),
];

const LEAD_DISTANCE_1: f32 = 10.0;
const LEAD_DISTANCE_2: f32 = 50.0;

const DRIBBLE_DIST_X: f32 = 18.0;
const DRIBBLE_DIST_Y: f32 = 16.0;

// Speeds for players in various situations. Speeds including 'BASE' can be boosted by the speed_boost difficulty
// setting (only for players on a computer-controlled team)
const PLAYER_DEFAULT_SPEED: f32 = 2.0;
const CPU_PLAYER_WITH_BALL_BASE_SPEED: f32 = 2.6;
const PLAYER_INTERCEPT_BALL_SPEED: f32 = 2.75;
const LEAD_PLAYER_BASE_SPEED: f32 = 2.9;
const HUMAN_PLAYER_WITH_BALL_SPEED: f32 = 3.0;
const HUMAN_PLAYER_WITHOUT_BALL_SPEED: f32 = 3.3;
const MAX_SPEED: f32 = 10.0;

const GOALS_TO_WIN: u8 = 9;

/*
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

impl Controls {
    fn movement(&self) -> Vector {
        let dy = if is_key_down(self.up) {
            -1.
        } else if is_key_down(self.down) {
            1.
        } else {
            0.
        };
        let dx = if is_key_down(self.left) {
            -1.
        } else if is_key_down(self.right) {
            1.
        } else {
            0.
        };
        return vec2(dx, dy) * MAX_SPEED;
    }
}

const TEAM_CONTROLS: [Controls; 2] = [
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
    holdoff_timer: i8,
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

#[derive(Copy, Clone)]
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

const ANGLE_DIFFS: [i32; 8] = [0, 1, 1, 1, 1, 7, 7, 7];

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

#[derive(Debug)]
enum ShootTarget {
    Goal(Position),
    Player(Position, Entity),
}

impl ShootTarget {
    fn position(&self) -> Position {
        match self {
            Self::Goal(p) => *p,
            Self::Player(p, _) => *p,
        }
    }
}

#[derive(Copy, Clone)]
enum Mark {
    Goal(Position),
    Player(Entity),
}

struct Lead(Option<f32>, Option<u8>);

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
    ball_owner: Option<Entity>,
    kickoff_player: Option<Entity>,
    teams: [TeamInfo; 2],
    scoring_team: usize,
    score_timer: i32,
    debug_shoot_target: Option<Vector>,
    shoot_now: [bool; 2],
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
            ball_owner: None,
            kickoff_player: None,
            teams: [TeamInfo::new(None), TeamInfo::new(None)],
            scoring_team: 1,
            score_timer: 0,
            debug_shoot_target: None,
            shoot_now: [false, false],
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
        self.ball_owner = None;
        self.camera_focus = vec2(HALF_LEVEL_W as f32, HALF_LEVEL_H as f32);
    }

    fn check_goals(&mut self) {
        let ball_y = self.world.get_mut::<Position>(self.ball).unwrap().0.y;
        self.score_timer -= 1;
        if self.score_timer == 0 {
            self.reset();
        } else if self.score_timer < 0 && (ball_y - HALF_LEVEL_H).abs() > HALF_PITCH_H {
            // todo play goal sound
            self.scoring_team = if ball_y < HALF_LEVEL_H { 0 } else { 1 };
            self.teams[self.scoring_team].score += 1;
            self.score_timer = 60;
        }
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
        for ii in 0..14 {
            self.world.insert_one(ids[ii], Peer(ids[13 - ii])).unwrap();
            self.world
                .insert_one(ids[ii], Mark::Player(ids[13 - ii]))
                .unwrap();
        }
        self.teams[0].active_player = Some(ids[0]);
        self.teams[1].active_player = Some(ids[1]);
        let kickoff_team = 1 - self.scoring_team;
        let kp = ids[kickoff_team];
        self.kickoff_player = Some(kp);
        self.world.get_mut::<Position>(kp).unwrap().0 = vec2(
            HALF_LEVEL_W - 30.0 + kickoff_team as f32 * 60.0,
            HALF_LEVEL_H,
        );
    }

    fn update(&mut self) {
        for (_, t) in &mut self.world.query::<&mut Timer>() {
            if t.0 > 0 {
                t.0 -= 1
            }
        }
        self.check_goals();
        self.set_behaviours();
        self.set_player_targets();
        update_players(&mut self.world, self.ball);
        self.update_ball();
        self.switch_players();
    }

    fn set_behaviours(&mut self) {
        for (_, (peer, mark, lead)) in self.world.query_mut::<(&Peer, &mut Mark, &mut Lead)>() {
            *mark = Mark::Player(peer.0);
            *lead = Lead(None, None);
        }
        match self.ball_owner {
            None => {}
            Some(owner_id) => {
                let ball_owner_pos = self.world.get::<Position>(owner_id).unwrap().0;
                let defending_team = 1 - self.world.get::<Team>(owner_id).unwrap().0;
                let goal = vec2(HALF_LEVEL_W, (1 - defending_team) as f32 * LEVEL_H);
                if self.difficulty.goalie_enabled {
                    // find the nearest player to the goal on each team and make them mark it
                    let goalie = self
                        .world
                        .query::<(&Team, &Position)>()
                        .iter()
                        .filter(|(_, (t, _))| t.0 == defending_team)
                        .min_by(|(_, (_, p1)), (_, (_, p2))| cmp_dist(p1.0, p2.0, goal))
                        .unwrap()
                        .0;
                    let ball_owner_peer = self.world.get::<Peer>(owner_id).unwrap().0;
                    let goalie_mark = *self.world.get_mut::<Mark>(goalie).unwrap();
                    *self.world.get_mut::<Mark>(ball_owner_peer).unwrap() = goalie_mark;
                    *self.world.get_mut::<Mark>(goalie).unwrap() = Mark::Goal(Position(goal));
                }
                // find the two leads
                let mut players = self
                    .world
                    .query::<(&Team, &Timer, &Mark, &Position)>()
                    .iter()
                    .filter(|(id, (team, timer, mark, _))| {
                        let dt = &self.teams[defending_team as usize];
                        team.0 == defending_team
                            && timer.0 <= 0
                            && (!dt.human()
                                || dt.active_player.is_none()
                                || dt.active_player.unwrap() != *id)
                            && match *mark {
                                Mark::Player(_) => true,
                                Mark::Goal(_) => false,
                            }
                    })
                    .map(|(id, (_, _, _, pos))| (id, pos.0))
                    .collect::<Vec<_>>();
                players.sort_by(|a, b| cmp_dist(a.1, b.1, ball_owner_pos));
                let (upfield, downfield): (Vec<(Entity, Vector)>, Vec<(Entity, Vector)>) =
                    players.iter().partition(|(_, pos)| {
                        if defending_team == 1 {
                            pos.y > ball_owner_pos.y
                        } else {
                            pos.y < ball_owner_pos.y
                        }
                    });
                let mut upfield: Vec<_> = upfield.into_iter().map(Some).collect();
                upfield.extend([None, None]);
                let mut downfield: Vec<_> = downfield.into_iter().map(Some).collect();
                downfield.extend([None, None]);
                use std::iter::once;
                let alternating: Vec<(Entity, Vector)> = upfield
                    .into_iter()
                    .zip(downfield.into_iter())
                    .flat_map(|tup| once(tup.0).chain(once(tup.1)))
                    .filter_map(|x| x)
                    .collect();
                for (n, (id, _)) in alternating.iter().enumerate() {
                    let mut lead = self.world.get_mut::<Lead>(*id).unwrap();
                    lead.1 = Some(n as u8);
                    if n == 0 {
                        lead.0 = Some(LEAD_DISTANCE_1);
                    }
                    if n == 1 && self.difficulty.second_lead_enabled {
                        lead.0 = Some(LEAD_DISTANCE_2);
                    }
                }
            }
        }
    }

    fn set_player_targets(&mut self) {
        for (id, (pos, team, home, lead, mark, target)) in
            &mut self
                .world
                .query::<(&Position, &Team, &Home, &Lead, &Mark, &mut Target)>()
        {
            // if we're pre-kickoff and not the kickoff player, just stand and wait
            if self.kickoff_player.is_some() && self.kickoff_player.unwrap() != id {
                target.pos = pos.0;
                continue;
            }
            let my_team = &self.teams[team.0 as usize];
            let i_am_active_player = match my_team.active_player {
                None => false,
                Some(aid) => aid == id,
            };
            let ball_pos = self.world.get::<Position>(self.ball).unwrap().0;
            let active = (ball_pos.y - pos.0.y).abs() < 400.0;
            // choose one of the following behaviours
            // if we're the currently-controlled player on a human team, respond to controls
            if my_team.human() && i_am_active_player {
                if self.ball_owner == Some(id) {
                    target.speed = HUMAN_PLAYER_WITH_BALL_SPEED;
                } else {
                    target.speed = HUMAN_PLAYER_WITHOUT_BALL_SPEED;
                }
                target.pos = pos.0 + my_team.controls.unwrap().movement();
                continue;
            }
            // set the default behaviour
            target.pos = home.0;
            target.speed = PLAYER_DEFAULT_SPEED;
            match self.ball_owner {
                Some(owner_id) if owner_id == id => {
                    // todo if we're computer-controlled and have the ball, do the cost function thing
                }
                Some(owner_id) => {
                    if team.0 == self.world.get::<Team>(owner_id).unwrap().0 {
                        // if my team has the ball and I'm active, go somewhere useful
                        if active {
                            let direction = if team.0 == 0 { -1. } else { 1. };
                            target.pos = (home.0 + (ball_pos + vec2(0.0, 400.0 * direction))) / 2.0;
                        }
                    } else {
                        match lead.0 {
                            Some(lead_dist) => {
                                // todo if other team has the ball and I'm a lead, try to intercept
                                let ball_owner_ref = self.world.entity(owner_id).unwrap();
                                let ball_owner_pos = ball_owner_ref.get::<Position>().unwrap().0;
                                let ball_owner_dir = ball_owner_ref.get::<Animation>().unwrap().dir;
                                let mut targ =
                                    ball_owner_pos + Angle::to_vec(ball_owner_dir) * lead_dist;
                                targ.x = targ.x.max(AI_MIN_X).min(AI_MAX_X);
                                targ.y = targ.y.max(AI_MIN_Y).min(AI_MAX_Y);
                                let other_team = &self.teams[1 - team.0 as usize];
                                target.pos = targ;
                                target.speed = LEAD_PLAYER_BASE_SPEED
                                    + if other_team.human() {
                                        self.difficulty.speed_boost
                                    } else {
                                        0.
                                    };
                            }
                            None => {
                                let mark_pos = match mark {
                                    Mark::Goal(goal_pos) => goal_pos.0,
                                    Mark::Player(mark_id) => {
                                        self.world.get::<Position>(*mark_id).unwrap().0
                                    }
                                };
                                // if our mark is inactive, do nothing (run towards home)
                                // if active, human team players just run towards the ball, computer
                                // players mark the designated target
                                if (mark_pos.y - ball_pos.y).abs() < 400. {
                                    if my_team.human() {
                                        target.pos = ball_pos;
                                    } else {
                                        let mark_to_ball_vec = ball_pos - mark_pos;
                                        let dist_from_mark = match mark {
                                            Mark::Goal(_) => mark_to_ball_vec.length().min(150.0),
                                            Mark::Player(_) => mark_to_ball_vec.length() / 2.0,
                                        };
                                        target.pos = mark_pos
                                            + mark_to_ball_vec.normalize() * dist_from_mark;
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    // if no-one has the ball and I'm active, try to intercept the ball
                    let mut sim_ball_pos = ball_pos;
                    let mut sim_ball_vel = *self.world.get::<Vector>(self.ball).unwrap();
                    let mut frame = 0.0;
                    while (sim_ball_pos - pos.0).length()
                        > PLAYER_INTERCEPT_BALL_SPEED * frame + DRIBBLE_DIST_X
                        && sim_ball_vel.length() > 0.5
                    {
                        sim_ball_pos += sim_ball_vel;
                        sim_ball_vel *= DRAG;
                        frame += 1.0;
                    }
                    target.pos = sim_ball_pos;
                    target.speed = PLAYER_INTERCEPT_BALL_SPEED;
                }
            }
        }
    }

    fn update_ball(&mut self) {
        let mut new_ball_vector = None;
        let mut ball_pos = self.world.get_mut::<Position>(self.ball).unwrap();
        let mut old_owner = None;
        let owner_team: Option<u8>;
        match self.ball_owner {
            None => {
                let bounds_x = if (ball_pos.0.y - HALF_LEVEL_H).abs() > HALF_PITCH_H {
                    GOAL_BOUNDS_X
                } else {
                    PITCH_BOUNDS_X
                };
                let bounds_y = if (ball_pos.0.x - HALF_LEVEL_W).abs() < HALF_GOAL_W {
                    GOAL_BOUNDS_Y
                } else {
                    PITCH_BOUNDS_Y
                };
                let vel = *self.world.get::<Vector>(self.ball).unwrap();
                let (px, vx) = ball_physics(ball_pos.0.x, vel.x, bounds_x);
                let (py, vy) = ball_physics(ball_pos.0.y, vel.y, bounds_y);
                ball_pos.0 = vec2(px, py);
                *self.world.get_mut::<Vector>(self.ball).unwrap() = vec2(vx, vy);
                owner_team = None;
            }
            Some(owner_id) => {
                self.kickoff_player = None;
                // calculate new position based on dribbling
                let owner_pos = &*self.world.get::<Position>(owner_id).unwrap();
                let owner_anim = &*self.world.get::<Animation>(owner_id).unwrap();
                let new_x = avg(
                    ball_pos.0.x,
                    owner_pos.0.x + DRIBBLE_DIST_X * owner_anim.dir.sin(),
                );
                let new_y = avg(
                    ball_pos.0.y,
                    owner_pos.0.y - DRIBBLE_DIST_Y * owner_anim.dir.cos(),
                );
                // check ball doesn't go off pitch
                if on_pitch(new_x, new_y) {
                    ball_pos.0 = vec2(new_x, new_y);
                } else {
                    // player dribbled off the pitch so they lose the ball
                    self.ball_owner = None;
                    self.world.get_mut::<Timer>(owner_id).unwrap().0 = 60;
                    new_ball_vector = Some(Angle::to_vec(owner_anim.dir) * 3.0);
                }
                owner_team = Some(self.world.get::<Team>(owner_id).unwrap().0);
            }
        }
        // update camera while we still have the ball position uniquely borrowed
        self.camera_focus += (ball_pos.0 - self.camera_focus).with_max_length(8.0);
        drop(ball_pos);
        // this is an awkward consequence of choosing to add and remove the Vector component
        if let Some(nbv) = new_ball_vector {
            self.world.insert_one(self.ball, nbv).unwrap();
        }
        let ball_pos = self.world.get::<Position>(self.ball).unwrap().0;
        // search for a player that can acquire the ball
        let mut ball_was_acquired = false;
        for (id, (player_pos, team, timer)) in &mut self.world.query::<(&Position, &Team, &Timer)>()
        {
            if (owner_team.is_none() || owner_team.unwrap() != team.0)
                && (ball_pos - player_pos.0).length() <= DRIBBLE_DIST_X
                && timer.0 == 0
            {
                old_owner = self.ball_owner;
                // acquire the ball
                self.ball_owner = Some(id);
                self.teams[team.0 as usize].active_player = Some(id);
                ball_was_acquired = true;
            }
        }
        if ball_was_acquired {
            if old_owner.is_none() {
                self.world.remove_one::<Vector>(self.ball).unwrap();
            }
            // set ball's timer so the computer can't shoot immediately
            let mut ball_timer = self.world.get_mut::<Timer>(self.ball).unwrap();
            ball_timer.0 = self.difficulty.holdoff_timer;
        }
        // if someone lost the ball, set their timer so they can't reacquire it
        old_owner.map(|owner| {
            let mut owner_timer = self.world.get_mut::<Timer>(owner).unwrap();
            owner_timer.0 = 60;
        });
        // if the ball has an owner, maybe kick it
        self.shoot_now = [false, false];
        self.debug_shoot_target = None;
        match self.ball_owner {
            None => (),
            Some(owner_id) => {
                let owner_team_id = self.world.get::<Team>(owner_id).unwrap().0;
                let owner_team = &self.teams[owner_team_id as usize];
                let owner_team_human = owner_team.human();
                let owner_pos = self.world.get::<Position>(owner_id).unwrap().0;
                let owner_dir = self.world.get::<Animation>(owner_id).unwrap().dir;
                // possible targets are all the other players on owner's team ...
                let mut targets: Vec<ShootTarget> = self
                    .world
                    .query::<(&Team, &Position)>()
                    .iter()
                    .filter(|(id, _)| id != &owner_id)
                    .filter(|(_, (t, _))| t.0 == owner_team_id)
                    .map(|(id, (_, p))| ShootTarget::Player(*p, id))
                    .collect();
                // ... plus the opposing goal
                // todo: if owner is a computer, filter out interceptable passes
                targets.push(ShootTarget::Goal(Position(vec2(
                    HALF_LEVEL_W,
                    owner_team_id as f32 * LEVEL_H,
                ))));
                targets.retain(|st| {
                    let shoot_vec = st.position().0 - owner_pos;
                    if shoot_vec.length() <= 0.0 || shoot_vec.length() >= 300.0 {
                        return false;
                    }
                    let source_dir = self.world.get::<Animation>(owner_id).unwrap().dir;
                    shoot_vec.normalize().dot(Angle::to_vec(source_dir)) > 0.8
                });
                let best_target = targets
                    .iter()
                    .min_by(|a, b| cmp_dist(a.position().0, b.position().0, owner_pos));
                self.debug_shoot_target = best_target.map(|st| st.position().0);
                let do_shoot;
                if owner_team.human() {
                    do_shoot = is_key_pressed(owner_team.controls.unwrap().shoot)
                } else {
                    // todo logic for when computer players shoot
                    do_shoot = false;
                }
                self.shoot_now[owner_team_id as usize] = do_shoot;
                if do_shoot {
                    let shoot_vec;
                    match best_target {
                        Some(t) => {
                            match t {
                                ShootTarget::Player(_, id) => {
                                    self.teams[owner_team_id as usize].active_player = Some(*id);
                                }
                                _ => (),
                            }
                            if owner_team_human
                                && matches!(best_target, Some(ShootTarget::Player(_, _)))
                            {
                                let mut lead = 0.0;
                                let mut targ = t.position().0;
                                for _ in 1..=8 {
                                    targ = t.position().0 + Angle::to_vec(owner_dir) * lead;
                                    let length = (targ - owner_pos).length();
                                    lead = HUMAN_PLAYER_WITHOUT_BALL_SPEED * steps(length) as f32;
                                }
                                shoot_vec = targ - owner_pos;
                            } else {
                                shoot_vec = t.position().0 - owner_pos;
                            }
                        }
                        None => {
                            shoot_vec =
                                Angle::to_vec(self.world.get::<Animation>(owner_id).unwrap().dir);
                            // take a guess at which player we should activate
                            let dest = owner_pos + shoot_vec.normalize() * 250.0;
                            let closest_player = self
                                .world
                                .query::<(&Team, &Position)>()
                                .iter()
                                .filter(|(_, (t, _))| t.0 == owner_team_id)
                                .map(|(id, (_, p))| (id, p))
                                .min_by(|a, b| cmp_dist((a.1).0, (b.1).0, dest))
                                .map(|(id, _)| id);
                            self.teams[owner_team_id as usize].active_player = closest_player;
                        }
                    }
                    self.world.get_mut::<Timer>(owner_id).unwrap().0 = 10;
                    self.ball_owner = None;
                    self.world
                        .insert_one(self.ball, shoot_vec.normalize() * KICK_STRENGTH)
                        .unwrap();
                }
            }
        }
    }

    fn switch_players(&mut self) {
        if self.kickoff_player.is_some() {
            return;
        }
        for t in 0..=1 {
            if !self.shoot_now[t] {
                if let Some(controls) = self.teams[t].controls {
                    if is_key_pressed(controls.shoot) {
                        let ball_pos = self.world.get::<Position>(self.ball).unwrap().0;
                        // switch control to the nearest player to the ball
                        let dir_bias = if self.ball_owner.is_some() {
                            2.0 * t as f32 - 1.0
                        } else {
                            0.0
                        };
                        self.teams[t].active_player = self
                            .world
                            .query::<(&Team, &Position)>()
                            .iter()
                            .filter(|(_, (tn, _))| tn.0 as usize == t)
                            .min_by(|a, b| {
                                cmp_dist_weighted(((a.1).1).0, ((b.1).1).0, ball_pos, dir_bias)
                            })
                            .map(|(id, _)| id);
                    }
                }
            }
        }
    }
}

fn build_ball(eb: &mut EntityBuilder) {
    eb.add(Position(vec2(HALF_LEVEL_W as f32, HALF_LEVEL_H as f32)));
    eb.add::<Vector>(vec2(0.0, 0.0));
    eb.add(Timer(0));
    eb.add(Ball);
}

fn build_player(eb: &mut EntityBuilder, x: f32, y: f32, offs: f32, team: u8) {
    let x = x + gen_range(-32., 32.);
    let y = y + gen_range(-32., 32.);
    eb.add(Home(vec2(x, y)));
    let start = vec2(x, y / 2. + offs);
    eb.add(Position(start.clone()));
    eb.add(Target::new(start));
    eb.add(Team(team));
    eb.add(Timer(0));
    eb.add(Animation::new());
    eb.add(Lead(None, None));
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
            if allow_movement(pos.0.x + vector.x, pos.0.y) {
                pos.0.x += vector.x;
            }
            if allow_movement(pos.0.x, pos.0.y + vector.y) {
                pos.0.y += vector.y;
            }
            anim.frame += vector.length().min(4.5);
            anim.frame %= 72.0;
        }
        let dir_diff = target_dir.0 - anim.dir.0;
        anim.dir = Angle((anim.dir.0 + ANGLE_DIFFS[dir_diff as usize % 8]) % 8);
    }
}

fn avg(a: f32, b: f32) -> f32 {
    if (b - a).abs() < 1.0 {
        b
    } else {
        (a + b) / 2.0
    }
}

fn ball_physics(pos: f32, vel: f32, bounds: (f32, f32)) -> (f32, f32) {
    let mut pos = pos;
    let mut vel = vel;
    pos += vel;
    if pos < bounds.0 || pos > bounds.1 {
        pos -= vel;
        vel = -vel;
    }
    (pos, vel * DRAG)
}

fn steps(distance: f32) -> i32 {
    if distance < 574.0 {
        ((1.0 - (distance * (1.0 - DRAG)) / KICK_STRENGTH).log(DRAG)).ceil() as i32
    } else {
        190 // ball comes to rest after 190 frames having travelled 574 pixels
    }
}

fn cmp_dist(v1: Vector, v2: Vector, dest: Vector) -> std::cmp::Ordering {
    (v1 - dest)
        .length()
        .partial_cmp(&(v2 - dest).length())
        .unwrap_or(std::cmp::Ordering::Equal)
}

fn cmp_dist_weighted(v1: Vector, v2: Vector, dest: Vector, bias: f32) -> std::cmp::Ordering {
    let l1 = (v1 - dest).length()
        / if (v1.y - dest.y) * bias < 0.0 {
            2.0
        } else {
            1.0
        };
    let l2 = (v2 - dest).length()
        / if (v2.y - dest.y) * bias < 0.0 {
            2.0
        } else {
            1.0
        };
    l1.partial_cmp(&l2).unwrap_or(std::cmp::Ordering::Equal)
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

fn allow_movement(x: f32, y: f32) -> bool {
    if (x - HALF_LEVEL_W).abs() > HALF_LEVEL_W {
        // Trying to walk off the left or right side of the level
        false
    } else if (x - HALF_LEVEL_W).abs() < HALF_GOAL_W + 20.0 {
        // Player is within the bounds of the goals on the X axis, don't let them walk into, through or behind the goal
        // +20 takes with of player sprite into account
        return (y - HALF_LEVEL_H).abs() < HALF_PITCH_H;
    } else {
        // Player is outside the bounds of the goals on the X axis, so they can walk off the pitch and to the edge
        // of the level
        (y - HALF_LEVEL_H).abs() < HALF_LEVEL_H
    }
}

struct Textures(HashMap<String, Texture2D>);

impl Textures {
    fn new() -> Self {
        return Self(HashMap::new());
    }
    async fn preload(&mut self, key: impl Into<String>) {
        let key: String = key.into();
        let texture = load_texture(&format!("images/{}.png", key)).await.unwrap();
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
    textures.preload("goal").await;
    textures.preload("goal0").await;
    textures.preload("goal1").await;
    textures.preload("bar").await;
    textures.preload("over0").await;
    textures.preload("over1").await;
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
    for k in 0..=9 {
        textures.preload(format!("s{}", k)).await;
        textures.preload(format!("l0{}", k)).await;
        textures.preload(format!("l1{}", k)).await;
    }
    // todo set up sound
    let mut state = State::Menu(MenuState::NumPlayers, Settings::new());
    let mut game = Game::new(get_difficulty(DifficultyLevel::Hard));
    let mut debug_draw = false;
    loop {
        match state {
            State::Menu(ref mut menu_state, ref mut settings) => {
                if is_key_pressed(KeyCode::Space) {
                    match menu_state {
                        MenuState::Difficulty => {
                            game = Game::new(get_difficulty(settings.difficulty_level));
                            game.teams[0].controls = Some(TEAM_CONTROLS[0]);
                            game.teams[1].controls = None;
                            state = State::Play;
                        }
                        MenuState::NumPlayers => match settings.num_players {
                            NumPlayers::One => {
                                *menu_state = MenuState::Difficulty;
                            }
                            NumPlayers::Two => {
                                game = Game::new(get_difficulty(DifficultyLevel::Hard));
                                game.teams[0].controls = Some(TEAM_CONTROLS[0]);
                                game.teams[1].controls = Some(TEAM_CONTROLS[1]);
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
                game.update();
            }
            State::Play => {
                if game.teams[0].score.max(game.teams[1].score) == GOALS_TO_WIN
                    && game.score_timer == 1
                {
                    state = State::GameOver;
                }
                game.update();
            }
            State::GameOver => {
                if is_key_pressed(KeyCode::Space) {
                    state = State::Menu(MenuState::NumPlayers, Settings::new());
                    Game::new(get_difficulty(DifficultyLevel::Hard));
                }
            }
        }

        if is_key_pressed(KeyCode::F1) {
            debug_draw = !debug_draw;
        }

        let offs_x = (game.camera_focus.x - WIDTH as f32 / 2.)
            .min(LEVEL_W - WIDTH)
            .max(0.0) as f32;
        let offs_y = (game.camera_focus.y - HEIGHT as f32 / 2.)
            .min(LEVEL_H - HEIGHT)
            .max(0.0) as f32;
        draw_texture(textures.get("pitch"), -offs_x, -offs_y, WHITE);

        let mut sprites: Vec<(String, f32, f32, f32)> = Vec::new();

        for (_id, (pos, team, anim)) in &mut game.world.query::<(&Position, &Team, &Animation)>() {
            let suffix = format!("{}{}", anim.dir.0, (anim.frame as u32 / 18));
            sprites.push((
                format!("player{}{}", team.0, suffix).to_owned(),
                pos.0.x - offs_x - 25., // hardcoded anchor
                pos.0.y - offs_y - 37., // hardcoded anchor
                pos.0.y,
            ));
            draw_texture(
                textures.get(&format!("players{}", suffix)),
                pos.0.x - offs_x - 25.,
                pos.0.y - offs_y - 37.,
                WHITE,
            );
        }

        // draw ball
        let ball_pos = &*game.world.get::<Position>(game.ball).unwrap();
        sprites.push((
            "ball".to_owned(),
            ball_pos.0.x - offs_x - 12.5,
            ball_pos.0.y - offs_y - 12.5,
            ball_pos.0.y,
        ));
        draw_texture(
            textures.get("balls"),
            ball_pos.0.x - offs_x - 12.5,
            ball_pos.0.y - offs_y - 12.5,
            WHITE,
        );

        // draw goals
        sprites.push((
            "goal0".to_owned(),
            HALF_LEVEL_W - offs_x - 100.0,
            0.0 - offs_y - 81.0,
            0.0,
        ));
        sprites.push((
            "goal1".to_owned(),
            HALF_LEVEL_W - offs_x - 100.0,
            LEVEL_H - offs_y - 125.0,
            LEVEL_H,
        ));

        sprites.sort_unstable_by(|(_, _, _, y1), (_, _, _, y2)| {
            y1.partial_cmp(y2).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (key, x, y, _) in sprites {
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
                draw_texture(textures.get("bar"), HALF_WINDOW_WIDTH - 176., 0., WHITE);
                for i in 0..=1 {
                    draw_texture(
                        textures.get(&format!("s{}", game.teams[i].score)),
                        HALF_WINDOW_WIDTH + 7. - 39. * i as f32,
                        6.,
                        WHITE,
                    );
                }
                if game.score_timer > 0 {
                    draw_texture(
                        textures.get("goal"),
                        HALF_WINDOW_WIDTH - 300.,
                        HEIGHT / 2. - 88.,
                        WHITE,
                    );
                }
            }
            State::GameOver => {
                draw_texture(
                    textures.get(if game.teams[0].score > game.teams[1].score {
                        "over0"
                    } else {
                        "over1"
                    }),
                    0.0,
                    0.0,
                    WHITE,
                );
                for i in 0..=1 {
                    draw_texture(
                        textures.get(&format!("l{}{}", i, game.teams[i].score)),
                        HALF_WINDOW_WIDTH + 25. - 125. * i as f32,
                        144.,
                        WHITE,
                    );
                }
            }
        }

        if debug_draw {
            draw_text("DEBUG MODE", 10., 10., 16., WHITE);
            // show player movement targets
            for (_, (pos, target)) in &mut game.world.query::<(&Position, &Target)>() {
                debug_draw_line(offs_x, offs_y, pos.0, target.pos, 1.0, RED);
            }
            // show shoot target
            match (game.debug_shoot_target, game.ball_owner) {
                (Some(v1), Some(owner_id)) => {
                    let v2 = game.world.get::<Position>(owner_id).unwrap().0;
                    debug_draw_line(offs_x, offs_y, v1, v2, 2.0, MAGENTA);
                }
                _ => (),
            }
            // show peers
            for (_, (pos, peer)) in &mut game.world.query::<(&Position, &Peer)>() {
                let peer_pos = game.world.get::<Position>(peer.0).unwrap();
                debug_draw_line(offs_x, offs_y, pos.0, peer_pos.0, 1.0, BLUE);
            }
            // show leads
            for (_, (pos, lead)) in &mut game.world.query::<(&Position, &Lead)>() {
                if let Lead(dist, Some(index)) = lead {
                    draw_text(
                        &format!("LEAD {}", index).to_owned(),
                        pos.0.x - offs_x + 15.0,
                        pos.0.y - offs_y,
                        24.0,
                        BLACK,
                    );
                    if dist.is_some() && game.ball_owner.is_some() {
                        let v2 = game
                            .world
                            .get::<Position>(game.ball_owner.unwrap())
                            .unwrap()
                            .0;
                        debug_draw_line(offs_x, offs_y, pos.0, v2, 2.0, BLACK)
                    }
                }
            }
        }

        next_frame().await;
    }
}

fn debug_draw_line(offs_x: f32, offs_y: f32, v1: Vector, v2: Vector, t: f32, c: Color) {
    draw_line(
        v1.x - offs_x,
        v1.y - offs_y,
        v2.x - offs_x,
        v2.y - offs_y,
        t,
        c,
    );
}
