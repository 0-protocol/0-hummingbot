//! Build script for 0-hummingbot
//!
//! Compiles Cap'n Proto schema files.

fn main() {
    // Compile trading schema
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/trading.capnp")
        .run()
        .expect("Failed to compile trading.capnp");
}
