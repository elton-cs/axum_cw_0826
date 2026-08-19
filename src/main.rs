pub mod game_core_fn;
pub mod game_core_ty;

use axum::{
    Router,
    extract::{Json, Query, State},
    http::StatusCode,
    routing::{any, get, post},
};
use game_core_fn::{ensure_tile_dimensions, update_game};
use game_core_ty::{
    Attempt, BOARD_HEIGHT, BOARD_WIDTH, FREE_GEM_GIFT, Game, GameCmd, GameError,
    MAX_PUZZLE_ATTEMPTS, Puzzle, Tile, User,
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{net::TcpListener, sync::watch};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

const GAME_STATE_PATH: &str = "game_state.json";
const SNAPSHOT_DIRECTORY: &str = "snapshots";
const SAVE_INTERVAL: Duration = Duration::from_secs(60);
const SNAPSHOT_INTERVAL_MINUTES: u64 = 5;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(load_game()?));

    let address = std::env::var("ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = TcpListener::bind(&address).await?;
    println!("Server running at http://{address}");

    let (save_shutdown_tx, save_shutdown_rx) = watch::channel(false);
    let save_task = tokio::spawn(periodically_save_game(state.clone(), save_shutdown_rx));

    axum::serve(listener, router(state.clone()))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Let an in-progress periodic save finish before writing the final state.
    let _ = save_shutdown_tx.send(true);
    save_task
        .await
        .map_err(|error| io::Error::other(format!("save task failed: {error}")))?;

    let game = state
        .lock()
        .map_err(|_| io::Error::other("game state lock poisoned"))?;
    save_game(&game)?;
    println!("Game state saved to {GAME_STATE_PATH}");
    Ok(())
}

fn load_game() -> Result<Game, Box<dyn std::error::Error>> {
    let mut game = if Path::new(GAME_STATE_PATH).exists() {
        println!("Restoring game state from {GAME_STATE_PATH}");
        serde_json::from_str(&std::fs::read_to_string(GAME_STATE_PATH)?)?
    } else {
        Game::default()
    };

    // Preserve existing placements while upgrading older 5 x 5 save files.
    ensure_tile_dimensions(&mut game, BOARD_WIDTH, BOARD_HEIGHT)
        .map_err(|error| std::io::Error::other(format!("failed to initialize game: {error:?}")))?;
    Ok(game)
}

fn save_game(game: &Game) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_vec_pretty(game)?;
    write_atomically(Path::new(GAME_STATE_PATH), &json)?;
    Ok(())
}

async fn periodically_save_game(state: GameState, mut shutdown: watch::Receiver<bool>) {
    let start = tokio::time::Instant::now() + SAVE_INTERVAL;
    let mut interval = tokio::time::interval_at(start, SAVE_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut minutes_elapsed = 0;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                minutes_elapsed += 1;
                let create_snapshot = minutes_elapsed % SNAPSHOT_INTERVAL_MINUTES == 0;
                if let Err(error) = persist_game(&state, create_snapshot).await {
                    eprintln!("Failed to save game state: {error}");
                }
            }
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    return;
                }
            }
        }
    }
}

async fn persist_game(state: &GameState, create_snapshot: bool) -> io::Result<()> {
    // Serialize a consistent state while holding the lock, then release it before disk I/O.
    let json = {
        let game = state
            .lock()
            .map_err(|_| io::Error::other("game state lock poisoned"))?;
        serde_json::to_vec_pretty(&*game).map_err(io::Error::other)?
    };

    tokio::task::spawn_blocking(move || {
        write_atomically(Path::new(GAME_STATE_PATH), &json)?;

        if create_snapshot {
            let snapshot_path = snapshot_path()?;
            write_atomically(&snapshot_path, &json)?;
            println!("Game state snapshot saved to {}", snapshot_path.display());
        }

        Ok::<(), io::Error>(())
    })
    .await
    .map_err(|error| io::Error::other(format!("save operation failed: {error}")))?
}

fn snapshot_path() -> io::Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    Ok(Path::new(SNAPSHOT_DIRECTORY).join(format!("game_state_{timestamp}.json")))
}

fn write_atomically(path: &Path, json: &[u8]) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }

    let temporary_path = path.with_extension("json.tmp");
    std::fs::write(&temporary_path, json)?;
    std::fs::rename(temporary_path, path)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install termination signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

pub fn router(state: GameState) -> Router {
    let frontend_dir = frontend_directory();
    let index_file = frontend_dir.join("index.html");
    let frontend = ServeDir::new(frontend_dir).fallback(ServeFile::new(index_file));

    Router::new()
        .route("/game/create-player", post(create_player))
        .route("/game/login", post(login))
        .route("/game/claim-free-gems", post(claim_free_gems))
        .route("/game/buy-puzzle", post(buy_puzzle))
        .route("/game/guess-puzzle", post(guess_puzzle))
        .route("/game/give-up-puzzle", post(give_up_puzzle))
        .route("/game/craft-single-rune", post(craft_single_rune))
        .route("/game/craft-random-rune", post(craft_random_rune))
        .route("/game/place-frag", post(place_frag))
        .route("/game/place-rune", post(place_rune))
        .route("/game/tiles", get(get_tiles))
        .route("/game/stats", get(get_game_stats))
        .route("/game/user", get(get_user))
        .route("/game/{*path}", any(api_not_found))
        .layer(CorsLayer::permissive())
        // Unknown non-API paths fall back to index.html so browser refreshes
        // work for every client-side route.
        .fallback_service(frontend)
        .with_state(state)
}

async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

fn frontend_directory() -> PathBuf {
    std::env::var_os("FRONTEND_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../tstack_cw_0826/dist"))
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
    exp_total: u32,
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
    attempt_word: [Option<Attempt>; MAX_PUZZLE_ATTEMPTS],
}

#[derive(Serialize)]
struct PreviousPuzzleResponse {
    reward_exp: u32,
    reward_frag: char,
    correct_word: String,
    attempt_word: [Option<Attempt>; MAX_PUZZLE_ATTEMPTS],
}

#[derive(Deserialize)]
struct LoginRequest {
    name: String,
    pass: String,
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
struct GiveUpPuzzleRequest {
    name: String,
    pass: String,
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

async fn login(State(state): State<GameState>, Json(request): Json<LoginRequest>) -> HandlerResult {
    authenticate(&state, &request.name, &request.pass)?;
    Ok(StatusCode::NO_CONTENT)
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

async fn give_up_puzzle(
    State(state): State<GameState>,
    Json(request): Json<GiveUpPuzzleRequest>,
) -> HandlerResult {
    let user_idx = authenticate(&state, &request.name, &request.pass)?;
    run(&state, GameCmd::GiveUpPuzzle { user_idx })
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
        exp_total: user.exp_total,
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
