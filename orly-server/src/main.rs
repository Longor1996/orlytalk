use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use serde::{Serialize, Deserialize};

use warp::{Filter, http::Response};

mod user;
use user::*;

mod websocket;
use websocket::*;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    
    let current_exe = std::env::current_exe().expect("Executable Location");
    let working_dir = current_exe.parent().expect("Working Directory");
    println!("Working Directory: {:?}", working_dir);
    
    let conn_path = working_dir.join("db.sqlite");
    println!("Database File: {:?}", conn_path);
    
    let conn = rusqlite::Connection::open(conn_path).expect("Failed to start SQLite!");
    
    conn.execute("
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER NOT NULL UNIQUE PRIMARY KEY AUTOINCREMENT,
            uuid BLOB NOT NULL UNIQUE,
            current_name TEXT NOT NULL
        );
    ", rusqlite::params![]).expect("SQLite Statement");
    
    // Clear out the cache...
    conn.flush_prepared_statement_cache();
    
    let users = Arc::new(Mutex::new(HashMap::new()));
    let users = warp::any().map(move || users.clone());
    
    let websocket = warp::path("websocket")
        .and(warp::path::end())
        .and(warp::query::<UserConnectionRequest>())
        .and(warp::ws())
        .and(users)
        .map(|ucr: UserConnectionRequest, ws: warp::ws::Ws, users| {
            ws.on_upgrade(move |socket| user_connected(socket, ucr, users))
    });
    
    fn static_reply(content_type: &str, body: &'static str) -> Result<warp::http::Response<&'static str>, warp::http::Error> {
        Response::builder()
            .header("Content-type", content_type)
            .body(body)
    }
    
    let index_html = warp::path::end().map(|| static_reply("text/html", include_str!("www/index.html")));
    let index_css = warp::path!("index.css").map(|| static_reply("text/css", include_str!("www/index.css")));
    
    let js_require = warp::path!("js" / "require.min.js").map(|| static_reply("application/javascript", include_str!("www/js/require.min.js")));
    let js_showdown = warp::path!("js" / "showdown.min.js").map(|| static_reply("application/javascript", include_str!("www/js/showdown.min.js")));
    let js_index = warp::path!("js" / "index.js").map(|| static_reply("application/javascript", include_str!("www/js/index.js")));
    let js_index_map = warp::path!("js" / "index.js.map").map(|| static_reply("application/javascript", include_str!("www/js/index.js.map")));
    
    let routes = index_html
        .or(index_css)
        .or(js_require)
        .or(js_showdown)
        .or(js_index)
        .or(js_index_map)
        .or(websocket)
        //.or(warp::fs::dir(working_dir.join("www")))
    ;
    
    let serve = warp::serve(routes);
    
    let ip = [0, 0, 0, 0];
    let port = 6991;
    
    println!("Socket-IP: {:?}", ip);
    println!("Socket-Port: {}", port);
    
    let addr = (ip, port);
    
    // Run forever!
    serve.run(addr).await;
}
