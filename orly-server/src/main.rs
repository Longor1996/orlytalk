pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use std::sync::Arc;

use warp::{Filter, http::Response};

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
    
    let db_file_name = std::env::var("ORLYTALK_SQLITE_FILE").unwrap_or_else(|_e| "db.sqlite".to_string());
    let db_file_path = working_dir.join(db_file_name);
    println!("Database File:     {:?}", db_file_path);
    
    let db_conn = rusqlite::Connection::open(db_file_path).expect("Failed to start SQLite!");
    
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
    
    let clients_map = Arc::new(DashMap::new());
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
    
    fn static_reply(content_type: &str, body: &'static [u8]) -> Result<warp::http::Response<&'static [u8]>, warp::http::Error> {
        Response::builder()
            .header("Content-type", content_type)
            .body(body)
    }
    
    let index_html = warp::path::end().map(|| static_reply("text/html", include_bytes!("www/index.html")));
    let index_css = warp::path!("index.css").map(|| static_reply("text/css", include_bytes!("www/index.css")));
    let favicon   = warp::path!("favicon.ico").map(|| static_reply("image/ico", include_bytes!("www/favicon.ico")));
    
    let js_require   = warp::path!("js" / "require.js").map(|| static_reply("application/javascript", include_bytes!("www/js/require.js")));
    let js_showdown  = warp::path!("js" / "showdown.js").map(|| static_reply("application/javascript", include_bytes!("www/js/showdown.js")));
    let js_index     = warp::path!("js" / "index.js").map(|| static_reply("application/javascript", include_bytes!("www/js/index.js")));
    let js_index_map = warp::path!("js" / "index.js.map").map(|| static_reply("application/javascript", include_bytes!("www/js/index.js.map")));
    
    let routes = index_html
        .or(index_css)
        .or(favicon)
        .or(js_require)
        .or(js_showdown)
        .or(js_index)
        .or(js_index_map)
        .or(websocket)
        //.or(warp::fs::dir(working_dir.join("www")))
    ;
    
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
