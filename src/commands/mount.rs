use std::path::Path;
use std::sync::{Arc, Mutex};
use fuse;

use config;
use api::{Client, DriveClient};
use cache::SqlCache;
use fs;
use chunk;

/// Execute starts the mount flow
pub fn execute(config_path: &str, mount_path: &str, uid: u32, gid: u32, threads: usize, chunk_size: u64, preload: u64) {
    let config_file_buf = Path::new(config_path).join("config.json");
    let token_file_buf = Path::new(config_path).join("token.json");
    let cache_file_buf = Path::new(config_path).join("cache.db");

    let config_file = config_file_buf.as_path();
    let token_file = token_file_buf.as_path();
    let cache_file = cache_file_buf.as_path();

    let config = match config::load(config_file.to_str().unwrap()) {
        Ok(config) => config,
        Err(_) => panic!("Could not read configuration"),
    };

    let drive_client = DriveClient::new(token_file.to_str().unwrap().to_owned(), config.client_id, config.client_secret);

    let cache = match SqlCache::new(cache_file.to_str().unwrap()) {
        Ok(cache) => Arc::new(Mutex::new(cache)),
        Err(cause) => panic!("{}", cause)
    };

    drive_client.watch_changes(cache.clone());

    let request_manager = match chunk::RequestManager::new(drive_client) {
        Ok(manager) => manager,
        Err(cause) => panic!("{}", cause)
    };

    let ram_manager = match chunk::RAMManager::new(request_manager) {
        Ok(manager) => manager,
        Err(cause) => panic!("{}", cause)
    };

    let thread_manager = match chunk::ThreadManager::new(ram_manager, threads) {
        Ok(manager) => manager,
        Err(cause) => panic!("{}", cause)
    };

    let preload_manager = match chunk::PreloadManager::new(thread_manager, preload, chunk_size) {
        Ok(manager) => manager,
        Err(cause) => panic!("{}", cause)
    };

    let filesystem = match fs::Filesystem::new(cache.clone(), preload_manager, uid, gid, chunk_size) {
        Ok(fs) => fs,
        Err(cause) => panic!("{}", cause)
    };

    info!("Mounting {}", mount_path);
    match fuse::mount(filesystem, &mount_path.to_owned(), &vec![]) {
        Ok(_) => info!("Unmounting {}", mount_path),
        Err(cause) => panic!("{}", cause)
    }
}