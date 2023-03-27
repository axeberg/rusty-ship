use std::collections::HashMap;

use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, Streaming};

use crate::battleship::{battleship_server::Battleship, Player};

use crate::battleship::{Guess, ShipPlacement};

const BOARD_SIZE: usize = 10;
const CARRIER_LENGTH: usize = 5;
const BATTLESHIP_LENGTH: usize = 4;
const CRUISER_LENGTH: usize = 3;
const SUBMARINE_LENGTH: usize = 3;
const DESTROYER_LENGTH: usize = 2;

#[derive(Debug, Clone)]
struct Cell {
    x: u32,
    y: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CellState {
    Empty,
    Occupied,
    Hit,
    Miss,
}

#[derive(Debug, Clone, Copy)]
enum ShipType {
    Carrier,
    Battleship,
    Cruiser,
    Submarine,
    Destroyer,
}

#[derive(Debug)]
enum Ship {
    Carrier(Vec<Cell>),
    Battleship(Vec<Cell>),
    Cruiser(Vec<Cell>),
    Submarine(Vec<Cell>),
    Destroyer(Vec<Cell>),
}

#[derive(Debug)]
struct PlayerState {
    id: String,
    ships: Vec<Ship>,
    guesses: Vec<(u32, u32)>,
}

impl PlayerState {
    fn new(id: String) -> Self {
        Self {
            id,
            ships: Vec::new(),
            guesses: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct GameStateInner {
    board: [[CellState; BOARD_SIZE]; BOARD_SIZE],
    ships_remaining: HashMap<ShipType, u32>,
    ship_positions: [[Option<ShipType>; BOARD_SIZE]; BOARD_SIZE],
    ship_cells: HashMap<ShipType, Vec<Cell>>,
    remaining_ships: Vec<ShipType>,
    turn: Option<Player>,
    events_tx: Sender<Event>,
}

#[derive(Debug)]
struct GameState {
    inner: Mutex<GameStateInner>,
}

#[derive(Debug)]
enum Event {
    PlayerJoined(Player),
    PlayerLeft(Player),
    PlayerTurn(Player),
    MakeGuess(Guess),
}


#[derive(Debug)]
pub struct BattleShipService {
    game_state: GameState,
}

impl BattleShipService {
    async fn new() -> Self {
        let (events_tx, events_rx) = channel(100);
        let game_state = GameState {
            inner: Mutex::new(GameStateInner {
                board: [[CellState::Empty; BOARD_SIZE]; BOARD_SIZE],
                ships_remaining: HashMap::new(),
                ship_positions: [[None; BOARD_SIZE]; BOARD_SIZE],
                ship_cells: HashMap::new(),
                remaining_ships: Vec::new(),
                turn: None,
                events_tx,
            }),
        };
        tokio::spawn(async move {
            while let Some(event) = events_rx.recv().await {
                match event {
                    Event::PlayerJoined(player) => {
                        println!("Player joined: {:?}", player);
                    }
                    Event::PlayerLeft(player) => {
                        println!("Player left: {:?}", player);
                    }
                    Event::PlayerTurn(player) => {
                        println!("Player turn: {:?}", player);
                    }
                    Event::MakeGuess(guess) => {
                        println!("Guess: {:?}", guess);
                    }
                }
            }
        });
        Self { game_state }
    }
}

#[tonic::async_trait]
impl Battleship for BattleShipService {
    async fn register_player(
        &self,
        request: Request<Player>,
    ) -> Result<Response<PlayerInfo>, Status> {
        let player = request.into_inner();
        let id = uuid::Uuid::new_v4().to_string();
        let player_info = PlayerInfo { id: id.clone() };
        let mut game_state = self.game_state.inner.lock().await;
        game_state
            .events_tx
            .send(Event::PlayerJoined(player.clone()))
            .await
            .unwrap();
        Ok(Response::new(player_info))
    }

    async fn remove_player(&self, request: Request<Player>) -> Result<Response<Empty>, Status> {
        let player = request.into_inner();
        let mut game_state = self.game_state.inner.lock().await;
        game_state
            .events_tx
            .send(Event::PlayerLeft(player.clone()))
            .await
            .unwrap();
        Ok(Response::new(Empty {}))
    }

    async fn place_ship(
        &self,
        request: tonic::Request<Streaming<ShipPlacement>>,
    ) -> Result<Response<Ship>, Status> {
        let mut stream = request.into_inner();
        let first_ship = stream
            .next()
            .await
            .ok_or_else(|| Status::invalid_argument("No ship placement in stream"))??;

        let mut game_state = self.game_state.inner.lock().await;
        let player_id = first_ship.player_id.clone();
        let player_state = game_state
            .ship_positions
            .get(&player_id)
            .unwrap_or_else(|| {
                game_state.ships_remaining.insert(ShipType::Carrier, 1);
                game_state.ships_remaining.insert(ShipType::Battleship, 1);
                game_state.ships_remaining.insert(ShipType::Cruiser, 1);
                game_state.ships_remaining.insert(ShipType::Submarine, 1);
                game_state.ships_remaining.insert(ShipType::Destroyer, 1);
                game_state
                    .remaining_ships
                    .push(ShipType::Carrier)
                    .push(ShipType::Battleship)
                    .push(ShipType::Cruiser)
                    .push(ShipType::Submarine)
                    .push(ShipType::Destroyer);
                game_state
                    .ship_positions
                    .insert(player_id.clone(), [[None; BOARD_SIZE]; BOARD_SIZE]);
                game_state
                    .ship_cells
                    .insert(ShipType::Carrier, Vec::new())
                    .insert(ShipType::Battleship, Vec::new())
                    .insert(ShipType::Cruiser, Vec::new())
                    .insert(ShipType::Submarine, Vec::new())
                    .insert(ShipType::Destroyer, Vec::new());
                PlayerState::new(player_id.clone())
            })
            .clone();

        for placement in stream {
            let placement = placement?;

            let ship_type = match placement.ship_type.as_str() {
                "carrier" => ShipType::Carrier,
                "battleship" => ShipType::Battleship,
                "cruiser" => ShipType::Cruiser,
                "submarine" => ShipType::Submarine,
                "destroyer" => ShipType::Destroyer,
                _ => return Err(Status::invalid_argument("Invalid ship type")),
            };

            if !game_state.remaining_ships.contains(&ship_type) {
                return Err(Status::invalid_argument(
                    "All ships of this type are already placed",
                ));
            }

            let (length, cells) = match ship_type {
                ShipType::Carrier => (CARRIER_LENGTH, &mut player_state.ships[0]),
                ShipType::Battles => (BATTLESHIP_LENGTH, &mut player_state.ships[1]),
                ShipType::Cruiser => (CRUISER_LENGTH, &mut player_state.ships[2]),
                ShipType::Submarine => (SUBMARINE_LENGTH, &mut player_state.ships[3]),
                ShipType::Destroyer => (DESTROYER_LENGTH, &mut player_state.ships[4]),
            };
        }
        Ok(Response::new({}));
    }

    fn make_guess(&self, request: Request<Guess>) -> Result<Response<Guess>, Status> {
        !todo!();
    }

    fn get_game_state(&self, request: Request<Player>) -> Result<Response<GameState>, Status> {
        !todo!();
    }
}
