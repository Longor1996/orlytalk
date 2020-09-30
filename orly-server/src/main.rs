#[macro_use]
extern crate lazy_static;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use std::sync::Arc;

use warp::{Filter};

use dashmap::DashMap;

mod user;
use user::*;

mod websocket;
use websocket::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
    println!("OrlyTalk Server");
    println!("- Version: {}", built_info::PKG_VERSION);
    println!("- For:     {}", built_info::TARGET);
    println!("- By:      {}", built_info::RUSTC_VERSION);
    println!("- In:      {}-mode", if built_info::DEBUG {"debug"} else {"release"});
    println!("- {}: {}",
        built_info::GIT_DIRTY.map(|b| if b {"Based on commit"} else {"From commit"}).unwrap_or("From commit"),
        built_info::GIT_COMMIT_HASH.unwrap_or("[HEAD]")
    );
    
    let current_exe = std::env::current_exe().expect("Executable Location");
    let working_dir = current_exe.parent().expect("Working Directory");
    println!("Working Directory: {:?}", working_dir);
    
    let db_file_name = std::env::var("ORLYTALK_SQLITE_FILE").unwrap_or_else(|_e| "./db.sqlite".to_string());
    let db_file_path = std::path::PathBuf::from(db_file_name);
    
    println!("Database File:     {:?}", db_file_path);
    println!("Database File:     {:?}", db_file_path.canonicalize());
    
    // If the database path has a parent directory, ensure it exists.
    if let Some(parent_dir) = db_file_path.parent() {
        std::fs::create_dir_all(parent_dir).expect("Could not create all directories for database");
    }
    
    let db_conn = rusqlite::Connection::open(&db_file_path).expect("Failed to start SQLite!");
    
    db_conn.execute("
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER NOT NULL UNIQUE PRIMARY KEY AUTOINCREMENT,
            uuid BLOB NOT NULL UNIQUE,
            current_name TEXT NOT NULL
        );
    ", rusqlite::params![]).expect("SQLite Statement");
    
    // Clear out the cache...
    db_conn.flush_prepared_statement_cache();
    
    let db_conn = Arc::new(tokio::sync::Mutex::from(db_conn));
    let db_conn = warp::any().map(move || db_conn.clone());
    
    let clients_map: Clients = Arc::new(DashMap::new());
    let clients_ref = warp::any().map(move || clients_map.clone());
    
    let websocket = warp::path("websocket")
        .and(warp::path::end())
        .and(warp::query::<UserConnectionRequest>())
        .and(warp::ws())
        .and(clients_ref)
        .and(db_conn)
        .map(|ucr: UserConnectionRequest, ws: warp::ws::Ws, users, _db_conn| {
            ws.on_upgrade(move |socket| client_connected(socket, ucr, users))
    });
    
    let www = warp::fs::dir(working_dir.join("orly-server-www"));
    let routes = websocket.or(www);
    
    let serve = warp::serve(routes);
    
    let host: std::net::IpAddr = std::env::var("ORLYTALK_HOST").unwrap_or_else(|_e| "0.0.0.0".to_string()).parse().expect("Valid host");
    let port: u16              = std::env::var("ORLYTALK_PORT").unwrap_or_else(|_e| "6991".to_string()).parse().expect("Valid port number");
    
    println!("Socket-Host: {:?}", host);
    println!("Socket-Port: {}", port);
    
    let addr = (host, port);
    
    // Run forever!
    println!();
    println!("Now running... (Press CTRL+C to kill the process!)");
    println!();
    serve.run(addr).await;
}
