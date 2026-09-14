//! Starquake, reimplemented in Rust.
//!
//! This crate contains no data from the original game. All graphics, maps
//! and text are read at startup from the player's own copy of the game (a
//! `.z80` snapshot), see [`assets`].

pub mod assets;
pub mod blob;
pub mod collide;
pub mod controls;
pub mod cores;
pub mod death;
pub mod display;
pub mod enemies;
pub mod entities;
pub mod entry;
pub mod flow;
pub mod game;
pub mod host;
pub mod hud;
pub mod layout;
pub mod map;
pub mod menu;
pub mod music;
pub mod newgame;
pub mod pickups;
pub mod play;
pub mod printer;
pub mod rng;
pub mod room;
pub mod screens;
pub mod sound;
pub mod sprites;

pub use game::Game;
