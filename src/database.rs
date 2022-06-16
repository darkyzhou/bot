use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref DATABASE: Arc<sled::Db> = Arc::new(
        sled::open("db").unwrap()
    );
}