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

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
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

#[derive(Deserialize)]
struct CreatePlayerRequest {
    name: String,
    pass: String,
}
#[derive(Deserialize)]
struct ClaimFreeGemsRequest {
    user_idx: usize,
    gem_gift: u32,
}
#[derive(Deserialize)]
struct UserRequest {
    user_idx: usize,
}
#[derive(Deserialize)]
struct GuessPuzzleRequest {
    user_idx: usize,
    guess_word: String,
}
#[derive(Deserialize)]
struct FragmentRequest {
    user_idx: usize,
    frag: char,
}
#[derive(Deserialize)]
struct RandomRuneRequest {
    user_idx: usize,
    frags: [char; 5],
}
#[derive(Deserialize)]
struct PlaceFragmentRequest {
    user_idx: usize,
    user_name: String,
    x: usize,
    y: usize,
    frag: char,
}
#[derive(Deserialize)]
struct PlaceRuneRequest {
    user_idx: usize,
    user_name: String,
    x: usize,
    y: usize,
    rune: char,
}

fn run(state: &GameState, command: GameCmd) -> HandlerResult {
    let result = update_game(
        &mut state.lock().expect("game state lock poisoned"),
        command,
    );
    result
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
    run(
        &state,
        GameCmd::ClaimFreeGems {
            user_idx: request.user_idx,
            gem_gift: request.gem_gift,
        },
    )
}

async fn buy_puzzle(
    State(state): State<GameState>,
    Json(request): Json<UserRequest>,
) -> HandlerResult {
    run(
        &state,
        GameCmd::BuyPuzzle {
            user_idx: request.user_idx,
        },
    )
}

async fn guess_puzzle(
    State(state): State<GameState>,
    Json(request): Json<GuessPuzzleRequest>,
) -> HandlerResult {
    run(
        &state,
        GameCmd::GuessPuzzle {
            user_idx: request.user_idx,
            guess_word: request.guess_word,
        },
    )
}

async fn craft_single_rune(
    State(state): State<GameState>,
    Json(request): Json<FragmentRequest>,
) -> HandlerResult {
    run(
        &state,
        GameCmd::CraftSingleRune {
            user_idx: request.user_idx,
            frag: request.frag,
        },
    )
}

async fn craft_random_rune(
    State(state): State<GameState>,
    Json(request): Json<RandomRuneRequest>,
) -> HandlerResult {
    run(
        &state,
        GameCmd::CraftRandomRune {
            user_idx: request.user_idx,
            frags: request.frags,
        },
    )
}

async fn place_frag(
    State(state): State<GameState>,
    Json(request): Json<PlaceFragmentRequest>,
) -> HandlerResult {
    run(
        &state,
        GameCmd::PlaceFrag {
            user_idx: request.user_idx,
            user_name: request.user_name,
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
    run(
        &state,
        GameCmd::PlaceRune {
            user_idx: request.user_idx,
            user_name: request.user_name,
            x: request.x,
            y: request.y,
            rune: request.rune,
        },
    )
}
