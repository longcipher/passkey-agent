//! BDD scenarios for pk-core library.
//!
//! Core library BDD scenarios are covered by unit tests.
//! Daemon state BDD scenarios are in pk-daemon/tests/bdd.rs.

use std::path::PathBuf;

use cucumber::World;

#[derive(Debug, Default, World)]
struct PkCoreWorld;

#[tokio::main]
async fn main() {
    let feature_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../features");
    PkCoreWorld::run(feature_path.as_path()).await;
}
