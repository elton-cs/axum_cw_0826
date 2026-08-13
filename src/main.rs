pub mod game_core_fn;
pub mod game_core_ty;

use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    routing::post,
};
use game_core_fn::update_game;
use game_core_ty::{Game, GameCmd, GameError};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let mut game = Game::default();
    update_game(&mut game, GameCmd::ServerPopulateTiles { x: 9, y: 9 }).unwrap();

    let address = "127.0.0.1:3000";
    let listener = TcpListener::bind(address).await?;
    println!("Server running at http://{address}");
    axum::serve(listener, router(Arc::new(Mutex::new(game)))).await
}

pub fn router(state: GameState) -> Router {
    Router::new()
        .route("/game/create-player", post(create_player))
        .route("/game/claim-free-gems", post(claim_free_gems))
        .route("/game/buy-puzzle", post(buy_puzzle))
        .route("/game/guess-puzzle", post(guess_puzzle))
        .route("/game/craft-single-rune", post(craft_single_rune))
        .route("/game/craft-random-rune", post(craft_random_rune))
        .route("/game/place-frag", post(place_frag))
        .route("/game/place-rune", post(place_rune))
        .with_state(state)
}

pub type GameState = Arc<Mutex<Game>>;
type HandlerResult = Result<StatusCode, (StatusCode, Json<GameError>)>;
type AuthResult = Result<usize, (StatusCode, Json<GameError>)>;

#[derive(Deserialize)]
struct CreatePlayerRequest {
    name: String,
    pass: String,
}

#[derive(Deserialize)]
struct ClaimFreeGemsRequest {
    name: String,
    pass: String,
    gem_gift: u32,
}

#[derive(Deserialize)]
struct BuyPuzzleRequest {
    name: String,
    pass: String,
}

#[derive(Deserialize)]
struct GuessPuzzleRequest {
    name: String,
    pass: String,
    guess_word: String,
}

#[derive(Deserialize)]
struct SingleRuneRequest {
    name: String,
    pass: String,
    frag: char,
}

#[derive(Deserialize)]
struct RandomRuneRequest {
    name: String,
    pass: String,
    frags: [char; 5],
}

#[derive(Deserialize)]
struct PlaceFragRequest {
    name: String,
    pass: String,
    x: usize,
    y: usize,
    frag: char,
}

#[derive(Deserialize)]
struct PlaceRuneRequest {
    name: String,
    pass: String,
    x: usize,
    y: usize,
    rune: char,
}

fn authenticate(state: &GameState, name: &str, pass: &str) -> AuthResult {
    let game = state.lock().expect("game state lock poisoned");
    let Some((user_idx, stored_pass)) = game.user_map.get(name) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(GameError::InvalidCredentials),
        ));
    };
    if stored_pass != pass {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(GameError::InvalidCredentials),
        ));
    }
    Ok(*user_idx)
}

fn run(state: &GameState, command: GameCmd) -> HandlerResult {
    update_game(
        &mut state.lock().expect("game state lock poisoned"),
        command,
    )
    .map(|_| StatusCode::OK)
    .map_err(|error| (StatusCode::BAD_REQUEST, Json(error)))
}

async fn create_player(
    State(state): State<GameState>,
    Json(request): Json<CreatePlayerRequest>,
) -> HandlerResult {
    run(
        &state,
        GameCmd::CreatePlayer {
            name: request.name,
            pass: request.pass,
        },
    )
}

async fn claim_free_gems(
    State(state): State<GameState>,
    Json(request): Json<ClaimFreeGemsRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(
        &state,
        GameCmd::ClaimFreeGems {
            user_idx,
            gem_gift: request.gem_gift,
        },
    )
}

async fn buy_puzzle(
    State(state): State<GameState>,
    Json(request): Json<BuyPuzzleRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(&state, GameCmd::BuyPuzzle { user_idx })
}

async fn guess_puzzle(
    State(state): State<GameState>,
    Json(request): Json<GuessPuzzleRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(
        &state,
        GameCmd::GuessPuzzle {
            user_idx,
            guess_word: request.guess_word,
        },
    )
}

async fn craft_single_rune(
    State(state): State<GameState>,
    Json(request): Json<SingleRuneRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(
        &state,
        GameCmd::CraftSingleRune {
            user_idx,
            frag: request.frag,
        },
    )
}

async fn craft_random_rune(
    State(state): State<GameState>,
    Json(request): Json<RandomRuneRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(
        &state,
        GameCmd::CraftRandomRune {
            user_idx,
            frags: request.frags,
        },
    )
}

async fn place_frag(
    State(state): State<GameState>,
    Json(request): Json<PlaceFragRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(
        &state,
        GameCmd::PlaceFrag {
            user_idx,
            user_name: request.name,
            x: request.x,
            y: request.y,
            frag: request.frag,
        },
    )
}

async fn place_rune(
    State(state): State<GameState>,
    Json(request): Json<PlaceRuneRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(
        &state,
        GameCmd::PlaceRune {
            user_idx,
            user_name: request.name,
            x: request.x,
            y: request.y,
            rune: request.rune,
        },
    )
}
