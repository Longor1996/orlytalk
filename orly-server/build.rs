use dircpy::copy_dir;
use std::env::var;
use std::path::PathBuf;
use std::fs::create_dir_all;

fn main() {
    built::write_built_file()
        .expect("Failed to acquire build-time information");
    
    let profile = var("PROFILE").expect("Name of Profile");
    let orlytalk_server_dir = var("CARGO_MANIFEST_DIR").expect("Path to CARGO_MANIFEST_DIR");
    
    let mut workspace = PathBuf::from(&orlytalk_server_dir);
    if !workspace.pop() {
        panic!("Can't determine workspace directory.");
    }
    let workspace = workspace;
    
    let mut www_src = workspace.clone();
    www_src.push("orly-client-web");
    www_src.push("out");
    
    let mut www_dst = workspace.clone();
    www_dst.push("target");
    www_dst.push(&profile);
    www_dst.push("www");
    
    println!("Workspace directory is {:?}", workspace);
    println!("WWW source directory is {:?}", www_src);
    println!("WWW target directory is {:?}", www_dst);
    
    create_dir_all(&www_src).expect("Could not create www source directory");
    create_dir_all(&www_dst).expect("Could not create www target directory");
    
    println!("Copying from {:?} to {:?}", www_src, www_dst);
    
    copy_dir(www_src, www_dst)
        .expect("Could not copy www directory to destination.");
}
