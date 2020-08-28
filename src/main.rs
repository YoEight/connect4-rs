#[macro_use]
extern crate serde_derive;

use serde::{ Serialize, Deserialize };
use chrono::{DateTime, Utc};

const HORIZONTAL_SLOT_COUNT: usize = 7;
const VERTICAL_SLOT_COUNT: usize = 6;
const SLOT_COUNT: usize = HORIZONTAL_SLOT_COUNT * VERTICAL_SLOT_COUNT;
const BOARD_POSITIONS: [Position; SLOT_COUNT] = board_positions();

type Column = usize;
type GameId = usize;

/*********************************************/
/*** Events                                  */
/*********************************************/
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GameCreated {
    player1: Player,
    player2: Player,
    created: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TokenPlaced {
    token: Token,
    column: usize,
    created: DateTime<Utc>,
}
/*********************************************/

/*********************************************/
/*** Commands                                */
/*********************************************/
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlaceToken {
    player: Player,
    column: usize,
}
/*********************************************/

/*********************************************/
/*** Validators                              */
/*********************************************/
fn is_valid_move(board: &Board, action: PlaceToken) -> bool {
    for pos in column_positions(action.column).iter() {
        let idx = pos.translate();

        if let Slot::Empty = board[idx] {
            return true;
        }
    }

    false
}

fn check_game_over<'a>(board: &Board, player1: &'a Player, player2: &'a Player) -> Option<&'a Player> {
    for pos in BOARD_POSITIONS.iter() {
        let slot = board[pos.translate()];

        match slot {
            Slot::Empty => continue,
            Slot::Occupied(token) => {
                let on_right_line =
                    pos.x + 3 < HORIZONTAL_SLOT_COUNT &&
                        board[pos.add_x(1).translate()] == slot &&
                        board[pos.add_x(2).translate()] == slot &&
                        board[pos.add_x(3).translate()] == slot;

                let on_top_line =
                    board[pos.add_y(1).translate()] == slot &&
                        board[pos.add_y(2).translate()] == slot &&
                        board[pos.add_y(3).translate()] == slot;

                let on_up_right_line =
                    pos.x + 3 < HORIZONTAL_SLOT_COUNT &&
                        board[pos.add_x(1).add_y(1).translate()] == slot &&
                        board[pos.add_x(2).add_y(2).translate()] == slot &&
                        board[pos.add_x(3).add_y(3).translate()] == slot;

                let on_up_left_line =
                    pos.x - 3 >= 0 &&
                        board[pos.sub_x(1).add_y(1).translate()] == slot &&
                        board[pos.sub_x(2).add_y(2).translate()] == slot &&
                        board[pos.sub_x(3).add_y(3).translate()] == slot;

                if on_right_line || (pos.y + 3 < VERTICAL_SLOT_COUNT && (on_top_line || on_up_right_line || on_up_left_line)) {
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
fn project_board(boards: &mut Board, event: &TokenPlaced) {
    for pos in column_positions(event.column).iter() {
        let idx = pos.translate();
        boards[idx] = Slot::Occupied(event.token);
    }
}

fn project_next_color_to_play(current: Token, event: &TokenPlaced) -> Token {
    match current {
        Token::Red => Token::Yellow,
        Token::Yellow => Token::Red,
    }
}

fn project_game_count(current: usize, event: &GameCreated) -> usize {
    current + 1
}
/*********************************************/
#[derive(Clone, Debug, Serialize, Deserialize)]
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

        Position {
            x,
            y,
        }
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

const fn empty_board() -> Board {
    [Slot::Empty; SLOT_COUNT]
}

const fn board_positions() -> [Position; SLOT_COUNT] {
    let init_pos = Position {
        x: 0,
        y: 0,
    };

    let mut positions = [init_pos; SLOT_COUNT];
    let mut idx = 0usize;
    let mut x = 0usize;

    while x < HORIZONTAL_SLOT_COUNT {
        let mut y = 0usize;
        while y < VERTICAL_SLOT_COUNT {
            positions[idx] = Position {
                x,
                y,
            };

            idx += 1;
            y += 1;
        }
        x += 1;
    }

    positions
}

fn column_positions(x: Column) -> [Position; VERTICAL_SLOT_COUNT] {
    let mut indexes = [Position { x: 0, y: 0 }; VERTICAL_SLOT_COUNT];
    let mut y = 0;

    while y < VERTICAL_SLOT_COUNT {
        indexes[y] = Position {x, y};
        y += 1;
    }

    indexes
}

#[test]
fn test_position_translation() {
    for pos in BOARD_POSITIONS.iter() {
        debug_assert_eq!(pos, &Position::from_index(pos.translate()));
    }
}

fn main() {
    let board: Board = empty_board();

    println!("Hello, world!");
}
