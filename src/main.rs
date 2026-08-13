pub mod game_core_fn;
pub mod game_core_ty;

use axum::Router;
use game_core_fn::update_game;
use game_core_ty::{Game, GameCmd};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let mut game = Game::default();

    update_game(&mut game, GameCmd::ServerPopulateTiles { x: 9, y: 9 })
        .expect("failed to populate game tiles");

    let game_state = Arc::new(Mutex::new(game));
    let app = Router::new().with_state(game_state);
    let listener = TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app).await
}
