#![allow(dead_code)]
#[macro_use]
extern crate serde_derive;

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::error::Error;
use eventstore::es6::connection::Connection;
use eventstore::es6::grpc::event_store::client::shared::Uuid;

const HORIZONTAL_SLOT_COUNT: usize = 7;
const VERTICAL_SLOT_COUNT: usize = 6;
const SLOT_COUNT: usize = HORIZONTAL_SLOT_COUNT * VERTICAL_SLOT_COUNT;

type Column = usize;
type GameId = usize;

/*********************************************/
/*** Events                                  */
/*********************************************/
#[derive(Clone, Debug, Serialize, Deserialize)]
enum GameEvents {
    GameCreated(GameCreated),
    TokenPlaced(TokenPlaced),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GameCreated {
    id: GameId,
    player1: Player,
    player2: Player,
    created: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TokenPlaced {
    game: GameId,
    token: Token,
    column: usize,
    created: DateTime<Utc>,
}
/*********************************************/

/*********************************************/
/*** Commands                                */
/*********************************************/
enum GameCommands {
    CreateGame(CreateGame),
    PlaceToken(PlaceToken),
}

struct CreateGame {
    player1: Player,
    player2: Player,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlaceToken {
    game: GameId,
    player: Player,
    column: usize,
}
/*********************************************/

/*********************************************/
/*** Validators                              */
/*********************************************/
fn is_valid_move(board: &Board, action: &PlaceToken) -> bool {
    for pos in column_positions(action.column).iter() {
        let idx = pos.translate();

        if let Slot::Empty = board[idx] {
            return true;
        }
    }

    false
}

fn can_create_game(games: &Games, command: &CreateGame) -> bool {
    for game in games.values() {
        if game.game_over().is_none() && (game.player1.name == command.player1.name
            || game.player1.name == command.player2.name
            || game.player2.name == command.player1.name
            || game.player2.name == command.player2.name)
        {
            return false;
        }
    }

    true
}

fn check_game_over<'a>(
    board: &Board,
    player1: &'a Player,
    player2: &'a Player,
) -> Option<&'a Player> {
    for pos in board_positions().iter() {
        let slot = board[pos.translate()];

        match slot {
            Slot::Empty => continue,
            Slot::Occupied(token) => {
                let on_right_line = pos.x + 3 < HORIZONTAL_SLOT_COUNT
                    && board[pos.add_x(1).translate()] == slot
                    && board[pos.add_x(2).translate()] == slot
                    && board[pos.add_x(3).translate()] == slot;

                let on_top_line = board[pos.add_y(1).translate()] == slot
                    && board[pos.add_y(2).translate()] == slot
                    && board[pos.add_y(3).translate()] == slot;

                let on_up_right_line = pos.x + 3 < HORIZONTAL_SLOT_COUNT
                    && board[pos.add_x(1).add_y(1).translate()] == slot
                    && board[pos.add_x(2).add_y(2).translate()] == slot
                    && board[pos.add_x(3).add_y(3).translate()] == slot;

                let on_up_left_line = pos.x >= 3
                    && board[pos.sub_x(1).add_y(1).translate()] == slot
                    && board[pos.sub_x(2).add_y(2).translate()] == slot
                    && board[pos.sub_x(3).add_y(3).translate()] == slot;

                if on_right_line
                    || (pos.y + 3 < VERTICAL_SLOT_COUNT
                        && (on_top_line || on_up_right_line || on_up_left_line))
                {
                    if player1.token == token {
                        return Some(player1);
                    } else {
                        return Some(player2);
                    }
                }
            }
        }
    }

    None
}
/*********************************************/
/*********************************************/
/*** Projections                             */
/*********************************************/
fn project_all_games(games: &mut Games, event: &GameEvents) {
    match event {
        GameEvents::GameCreated(event) => {
            let game = Game {
                id: event.id,
                player1: event.player1.clone(),
                player2: event.player2.clone(),
                board: empty_board(),
            };

            games.insert(event.id, game);
        }

        GameEvents::TokenPlaced(event) => {
            if let Some(game) = games.get_mut(&event.game) {
                project_board(&mut game.board, event)
            }
        }
    }
}

fn project_board(boards: &mut Board, event: &TokenPlaced) {
    for pos in column_positions(event.column).iter() {
        let idx = pos.translate();

        if let Slot::Empty = boards[idx] {
            boards[idx] = Slot::Occupied(event.token);
            break;
        }
    }
}

fn project_next_color_to_play(current: Token, _event: &TokenPlaced) -> Token {
    match current {
        Token::Red => Token::Yellow,
        Token::Yellow => Token::Red,
    }
}

fn project_game_count(current: usize, _event: &GameCreated) -> usize {
    current + 1
}
/*********************************************/
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Player {
    name: String,
    token: Token,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Position {
    x: usize,
    y: usize,
}

impl Position {
    pub fn translate(&self) -> usize {
        self.x + HORIZONTAL_SLOT_COUNT * self.y
    }

    pub fn from_coord(x: usize, y: usize) -> Self {
        Position { x, y }
    }

    pub fn from_index(idx: usize) -> Self {
        let mut x = idx;
        let mut y = 0;

        loop {
            if x < HORIZONTAL_SLOT_COUNT {
                break;
            }

            x -= HORIZONTAL_SLOT_COUNT;
            y += 1;
        }

        Position { x, y }
    }

    pub fn add_x(self, i: usize) -> Self {
        Position {
            x: self.x + i,
            ..self
        }
    }

    pub fn sub_x(self, i: usize) -> Self {
        Position {
            x: self.x - i,
            ..self
        }
    }

    pub fn add_y(self, i: usize) -> Self {
        Position {
            y: self.y + i,
            ..self
        }
    }

    pub fn sub_y(self, i: usize) -> Self {
        Position {
            y: self.y + i,
            ..self
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
enum Token {
    Red,
    Yellow,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Slot {
    Empty,
    Occupied(Token),
}

impl Slot {
    pub fn token(self) -> Option<Token> {
        match self {
            Slot::Empty => None,
            Slot::Occupied(p) => Some(p),
        }
    }
}

type Board = [Slot; SLOT_COUNT];

enum GameStatus {
    Ongoing,
    Terminated,
}

struct Game {
    id: GameId,
    player1: Player,
    player2: Player,
    board: Board,
}

impl Game {
    pub fn game_over(&self) -> Option<&Player> {
        for pos in board_positions().iter() {
            let slot = self.board[pos.translate()];

            match slot {
                Slot::Empty => continue,
                Slot::Occupied(token) => {
                    let on_right_line = pos.x + 3 < HORIZONTAL_SLOT_COUNT
                        && self.board[pos.add_x(1).translate()] == slot
                        && self.board[pos.add_x(2).translate()] == slot
                        && self.board[pos.add_x(3).translate()] == slot;

                    let on_top_line = self.board[pos.add_y(1).translate()] == slot
                        && self.board[pos.add_y(2).translate()] == slot
                        && self.board[pos.add_y(3).translate()] == slot;

                    let on_up_right_line = pos.x + 3 < HORIZONTAL_SLOT_COUNT
                        && self.board[pos.add_x(1).add_y(1).translate()] == slot
                        && self.board[pos.add_x(2).add_y(2).translate()] == slot
                        && self.board[pos.add_x(3).add_y(3).translate()] == slot;

                    let on_up_left_line = pos.x >= 3
                        && self.board[pos.sub_x(1).add_y(1).translate()] == slot
                        && self.board[pos.sub_x(2).add_y(2).translate()] == slot
                        && self.board[pos.sub_x(3).add_y(3).translate()] == slot;

                    if on_right_line
                        || (pos.y + 3 < VERTICAL_SLOT_COUNT
                        && (on_top_line || on_up_right_line || on_up_left_line))
                    {
                        if self.player1.token == token {
                            return Some(&self.player1);
                        } else {
                            return Some(&self.player2);
                        }
                    }
                }
            }
        }

        None
    }
}

type GameStatues = HashMap<GameId, GameStatus>;
type Boards = HashMap<GameId, Board>;
type Games = HashMap<GameId, Game>;

const fn empty_board() -> Board {
    [Slot::Empty; SLOT_COUNT]
}

fn board_positions() -> [Position; SLOT_COUNT] {
    let init_pos = Position { x: 0, y: 0 };

    let mut positions = [init_pos; SLOT_COUNT];

    for x in 0..HORIZONTAL_SLOT_COUNT {
        for y in 0..VERTICAL_SLOT_COUNT {
            let pos = Position { x, y };

            positions[pos.translate()] = pos;
        }
    }

    positions
}

fn column_positions(x: Column) -> [Position; VERTICAL_SLOT_COUNT] {
    let mut indexes = [Position { x: 0, y: 0 }; VERTICAL_SLOT_COUNT];

    for y in 0..VERTICAL_SLOT_COUNT {
        indexes[y] = Position { x, y };
    }

    indexes
}

#[test]
fn test_position_translation() {
    for pos in board_positions().iter() {
        debug_assert_eq!(pos, &Position::from_index(pos.translate()));
    }
}

#[test]
fn test_check_position_translate_idx() {
    let mut board = empty_board();
    for pos in board_positions().iter() {
        board[pos.translate()] = Slot::Occupied(Token::Red);
    }
}

#[test]
fn test_check_column_position_translate_idx() {
    let mut board = empty_board();
    for column in 0..HORIZONTAL_SLOT_COUNT {
        for pos in column_positions(column).iter() {
            board[pos.translate()] = Slot::Occupied(Token::Red);
        }
    }
}

#[test]
fn test_no_winner_empty_board() {
    let player1 = Player {
        token: Token::Red,
        name: "1".to_string(),
    };

    let player2 = Player {
        token: Token::Yellow,
        name: "2".to_string(),
    };

    debug_assert_eq!(None, check_game_over(&empty_board(), &player1, &player2));
}

#[test]
fn test_detect_win_condition_horizontal() {
    let mut events = Vec::new();
    let game = 0;

    for column in 0..4 {
        events.push(TokenPlaced {
            game,
            token: Token::Red,
            column,
            created: Utc::now(),
        });

        if column != 3 {
            events.push(TokenPlaced {
                game,
                token: Token::Yellow,
                column,
                created: Utc::now(),
            });
        }
    }

    let mut board = empty_board();
    let player1 = Player {
        token: Token::Red,
        name: "1".to_string(),
    };

    let player2 = Player {
        token: Token::Yellow,
        name: "2".to_string(),
    };

    for event in events.iter() {
        project_board(&mut board, event);
    }

    debug_assert_eq!(Some(&player1), check_game_over(&board, &player1, &player2));
}

#[test]
fn test_detect_win_condition_vertical() {
    let mut events = Vec::new();
    let game = 0;

    for round in 0..4 {
        events.push(TokenPlaced {
            game,
            token: Token::Red,
            column: 0,
            created: Utc::now(),
        });

        if round != 3 {
            events.push(TokenPlaced {
                game,
                token: Token::Yellow,
                column: 1,
                created: Utc::now(),
            });
        }
    }

    let mut board = empty_board();
    let player1 = Player {
        token: Token::Red,
        name: "1".to_string(),
    };

    let player2 = Player {
        token: Token::Yellow,
        name: "2".to_string(),
    };

    for event in events.iter() {
        project_board(&mut board, event);
    }

    debug_assert_eq!(Some(&player1), check_game_over(&board, &player1, &player2));
}

fn command_processing(games: &Games, cmd: GameCommands) -> Option<GameEvents> {
    match cmd {
        GameCommands::CreateGame(params) => {
            if can_create_game(games, &params) {
                let id = games.len();
                return Some(GameEvents::GameCreated(GameCreated {
                    id,
                    player1: params.player1.clone(),
                    player2: params.player2.clone(),
                    created: Utc::now(),
                }));
            }
        }

        GameCommands::PlaceToken(params) => {
            if let Some(game) = games.get(&params.game) {
                if is_valid_move(&game.board, &params) {
                    return Some(GameEvents::TokenPlaced(TokenPlaced {
                        game: params.game,
                        token: params.player.token,
                        column: params.column,
                        created: Utc::now(),
                    }));
                }
            }
        }
    }

    None
}

fn event_processing(games: &mut Games, event: &GameEvents) {
    match event {
        GameEvents::GameCreated(params) => {
            let game = Game {
                id: params.id,
                player1: params.player1.clone(),
                player2: params.player2.clone(),
                board: empty_board(),
            };

            games.insert(params.id, game);
        }

        GameEvents::TokenPlaced(params) => {
            if let Some(game) = games.get_mut(&params.game) {
                project_board(&mut game.board, params);
            }
        }
    }
}

// fn game_loop() {
//     let mut games: Games = HashMap::new();
//     loop {
//         let cmd = wait_for_game_command();

//         if let Some(event) = command_processing(&games, cmd) {
//             event_processing(&mut games, &event);
//             persist_event(&event);

//             if let GameEvents::TokenPlaced(event) = event {
//                 let game = games.get(&event.game).expect("Guaranteed the game exists");

//                 draw_board(&game.board);

//                 if let Some(player) = game.is_over() {
//                     notify_winner(player);

//                     break;
//                 }
//             }
//         }
//     }
// }

// struct State;

// async fn load_game_events(connection: &Connection) {
//     connection.read_stream("$ce-games".to_string())
//         .start_from_beginning();
// }

// struct ItemAdded {
//     id: Uuid,
//     name: String,
// }

// fn draw_board(board: &Board) {

// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let uri = format!("https://localhost:2113/").parse()?;
    // let connection = eventstore::es6::connection::Connection::builder()
    //     .with_default_user(eventstore::Credentials::new("admin", "changeit"))
    //     .disable_server_certificate_validation()
    //     .single_node_connection(uri)
    //     .await?;

    // let _board: Board = empty_board();
    // let _events: Vec<TokenPlaced> = Vec::new();

    // for pos in board_positions().iter() {
    //     println!("({}, {})", pos.x, pos.y);
    // }

    println!("Hello, world!");

    Ok(())
}
