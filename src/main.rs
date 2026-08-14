pub mod game_core_fn;
pub mod game_core_ty;

use axum::{
    Router,
    extract::{Json, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use game_core_fn::update_game;
use game_core_ty::{Attempt, FREE_GEM_GIFT, Game, GameCmd, GameError, Puzzle, Tile, User};
use serde::{Deserialize, Serialize};
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
        .route("/game/tiles", get(get_tiles))
        .route("/game/stats", get(get_game_stats))
        .route("/game/user", get(get_user))
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
struct UserQuery {
    name: String,
}

#[derive(Serialize)]
struct GameStatsResponse {
    protocol_gem: u32,
    treasury_gem: u32,
    games_bought: u32,
}

#[derive(Serialize)]
struct UserResponse {
    name: String,
    gem: u32,
    exp: u32,
    exp_next: u32,
    lvl: u32,
    frag_count: [u32; 26],
    rune_count: [u32; 26],
    curr_puzzle: Option<CurrentPuzzleResponse>,
    prev_puzzle: Vec<PreviousPuzzleResponse>,
}

#[derive(Serialize)]
struct CurrentPuzzleResponse {
    reward_exp: u32,
    reward_frag: char,
    attempt_word: Vec<Attempt>,
}

#[derive(Serialize)]
struct PreviousPuzzleResponse {
    reward_exp: u32,
    reward_frag: char,
    correct_word: String,
    attempt_word: Vec<Attempt>,
}

#[derive(Deserialize)]
struct ClaimFreeGemsRequest {
    name: String,
    pass: String,
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

async fn get_tiles(State(state): State<GameState>) -> Json<Vec<Vec<Tile>>> {
    let game = state.lock().expect("game state lock poisoned");
    Json(game.tile.clone())
}

async fn get_game_stats(State(state): State<GameState>) -> Json<GameStatsResponse> {
    let game = state.lock().expect("game state lock poisoned");
    Json(GameStatsResponse {
        protocol_gem: game.protocol_gem,
        treasury_gem: game.treasury_gem,
        games_bought: game.games_bought,
    })
}

async fn get_user(
    State(state): State<GameState>,
    Query(request): Query<UserQuery>,
) -> Result<Json<UserResponse>, (StatusCode, Json<GameError>)> {
    let game = state.lock().expect("game state lock poisoned");
    let Some((user_idx, _)) = game.user_map.get(&request.name) else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(GameError::PlayerNotFoundByName { name: request.name }),
        ));
    };
    let Some(user) = game.user.get(*user_idx) else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(GameError::PlayerNotFound {
                user_idx: *user_idx,
            }),
        ));
    };

    Ok(Json(user_response(user)))
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
            gem_gift: FREE_GEM_GIFT,
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

fn user_response(user: &User) -> UserResponse {
    UserResponse {
        name: user.name.clone(),
        gem: user.gem,
        exp: user.exp,
        exp_next: user.exp_next,
        lvl: user.lvl,
        frag_count: user.frag_count,
        rune_count: user.rune_count,
        curr_puzzle: user.curr_puzzle.as_ref().map(current_puzzle_response),
        prev_puzzle: user
            .prev_puzzle
            .iter()
            .map(previous_puzzle_response)
            .collect(),
    }
}

fn current_puzzle_response(puzzle: &Puzzle) -> CurrentPuzzleResponse {
    CurrentPuzzleResponse {
        reward_exp: puzzle.reward_exp,
        reward_frag: puzzle.reward_frag,
        attempt_word: puzzle.attempt_word.clone(),
    }
}

fn previous_puzzle_response(puzzle: &Puzzle) -> PreviousPuzzleResponse {
    PreviousPuzzleResponse {
        reward_exp: puzzle.reward_exp,
        reward_frag: puzzle.reward_frag,
        correct_word: puzzle.correct_word.clone(),
        attempt_word: puzzle.attempt_word.clone(),
    }
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
