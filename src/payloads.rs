//! What crosses the network: the server tells each player their vein and tree sizes, so the
//! client outlines what a break would take. Registered (version "1", optional) in `register`.
use mod_api::{PayloadCodec, ToClient, VarInt};
use std::sync::OnceLock;

/// Server → client: this player's comforts that change what a break takes.
#[derive(PayloadCodec, Clone, Debug, PartialEq)]
pub struct Caps {
    pub vein: VarInt,
    pub timber: VarInt,
}

pub static CAPS: OnceLock<ToClient<Caps>> = OnceLock::new();
