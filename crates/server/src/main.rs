//! HyperTide backend server

mod api;
mod bootstrap;
mod core;
mod health;
mod routes;
mod state;

#[cfg(test)]
mod tests;

pub use state::AppState;

#[tokio::main]
async fn main() {
    bootstrap::run().await;
}
