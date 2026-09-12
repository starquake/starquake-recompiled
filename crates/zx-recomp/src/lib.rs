//! zx-recomp: static recompiler from ZX Spectrum 48K Z80 code to Rust.
//!
//! Pipeline:
//! 1. Load the user's snapshot (and ROM) and check them against the hashes
//!    in the game config.
//! 2. Trace: run the game headless in the interpreter with scripted input,
//!    recording executed code, jump targets and self-modifying code.
//! 3. Analyse: recursive descent from every known entry point, then split
//!    the code into blocks.
//! 4. Generate Rust, one function per block, for the runtime to dispatch to.
//!
//! Typically driven from a game crate's `build.rs` via [`build_game`].

pub mod analysis;
pub mod codegen;
pub mod config;
pub mod listing;
pub mod tracer;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use zx_core::Snapshot;
use zx_core::sha1::sha1_hex;

pub use config::Config;

/// The user-supplied files a build works from.
pub struct Inputs {
    pub snapshot: Snapshot,
    pub snapshot_sha1: String,
    pub rom: Option<Vec<u8>>,
    pub rom_sha1: Option<String>,
}

fn read(path: &Path, what: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| {
        format!(
            "cannot read {what} {}: {e}\n\
             Copy your own copy of the file into the assets directory (see assets/README.md).",
            path.display()
        )
    })
}

fn check_hash(what: &str, actual: &str, expected: Option<&str>) -> Result<(), String> {
    match expected {
        Some(e) if !e.eq_ignore_ascii_case(actual) => Err(format!(
            "{what} has SHA-1 {actual}, but this config was written for {e}.\n\
             Use the dump the config expects, or update the hash in the config \
             (other dumps may need different analysis hints)."
        )),
        _ => Ok(()),
    }
}

impl Inputs {
    pub fn load(cfg: &Config, assets: &Path) -> Result<Inputs, String> {
        let snap_path = assets.join(&cfg.game.snapshot);
        let snap_bytes = read(&snap_path, "snapshot")?;
        let snapshot_sha1 = sha1_hex(&snap_bytes);
        check_hash("snapshot", &snapshot_sha1, cfg.game.snapshot_sha1.as_deref())?;
        let snapshot = zx_core::snapshot::load_z80(&snap_bytes)
            .map_err(|e| format!("{}: {e}", snap_path.display()))?;

        let (rom, rom_sha1) = match &cfg.game.rom {
            Some(name) => {
                let bytes = read(&assets.join(name), "ROM")?;
                if bytes.len() != 0x4000 {
                    return Err(format!("ROM {name} is {} bytes, expected 16384", bytes.len()));
                }
                let hash = sha1_hex(&bytes);
                check_hash("ROM", &hash, cfg.game.rom_sha1.as_deref())?;
                (Some(bytes), Some(hash))
            }
            None => (None, None),
        };
        Ok(Inputs {
            snapshot,
            snapshot_sha1,
            rom,
            rom_sha1,
        })
    }

    /// Memory image the analysis starts from: RAM from the snapshot, plus the ROM.
    pub fn memory(&self) -> Vec<u8> {
        let mut mem = self.snapshot.memory();
        if let Some(rom) = &self.rom {
            mem[..0x4000].copy_from_slice(rom);
        }
        mem
    }
}

/// Reads a miss log written by the runtime: one hex address per line.
pub fn read_misses(path: &Path) -> Vec<u16> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter_map(|w| u16::from_str_radix(w.trim_start_matches("0x"), 16).ok())
        .collect()
}

pub struct Output {
    pub source: String,
    pub report: String,
}

pub fn recompile(cfg: &Config, inputs: &Inputs, extra_entries: &[u16]) -> Result<Output, String> {
    let traced = tracer::run(&cfg.trace, inputs, |_, _| {})?;
    let analysis = analysis::analyze(
        cfg,
        &inputs.memory(),
        inputs.snapshot.pc,
        inputs.rom.is_some(),
        &traced.trace,
        extra_entries,
    );
    let meta = codegen::Meta {
        name: &cfg.game.name,
        snapshot_sha1: &inputs.snapshot_sha1,
        rom_sha1: inputs.rom_sha1.as_deref(),
    };
    let source = codegen::generate(&meta, &analysis);
    Ok(Output {
        source,
        report: report(&analysis),
    })
}

pub fn report(a: &analysis::Analysis) -> String {
    let s = &a.stats;
    let mut r = String::new();
    let _ = writeln!(r, "traced instructions:   {}", s.traced_instrs);
    let _ = writeln!(r, "traced entry points:   {}", s.traced_entries);
    let _ = writeln!(r, "blocks generated:      {}", s.entries);
    let _ = writeln!(r, "instructions compiled: {}", s.instrs);
    let _ = writeln!(r, "self-modifying instrs: {}", s.self_modifying.len());
    for a in &s.self_modifying {
        let _ = writeln!(r, "  {a:04x}");
    }
    r
}

/// Entry point for a game crate's `build.rs`: recompiles the game described
/// by `config_path` into `$OUT_DIR/recompiled.rs`.
///
/// Assets are looked up in `$ZX_ASSETS` if set, else in `default_assets`.
pub fn build_game(config_path: &str, default_assets: &str, misses_file: &str) {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let config_path = manifest.join(config_path);
    let assets = std::env::var_os("ZX_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join(default_assets));
    println!("cargo:rerun-if-env-changed=ZX_ASSETS");
    println!("cargo:rerun-if-changed={}", config_path.display());

    let fail = |msg: String| -> ! {
        for line in msg.lines() {
            println!("cargo:warning={line}");
        }
        panic!("zx-recomp: {msg}");
    };

    let text = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", config_path.display())));
    let cfg = Config::parse(&text).unwrap_or_else(|e| fail(format!("{}: {e}", config_path.display())));

    println!("cargo:rerun-if-changed={}", assets.join(&cfg.game.snapshot).display());
    if let Some(rom) = &cfg.game.rom {
        println!("cargo:rerun-if-changed={}", assets.join(rom).display());
    }
    let misses_path = assets.join(misses_file);
    println!("cargo:rerun-if-changed={}", misses_path.display());

    let inputs = Inputs::load(&cfg, &assets).unwrap_or_else(|e| fail(e));
    let misses = read_misses(&misses_path);
    let out = recompile(&cfg, &inputs, &misses).unwrap_or_else(|e| fail(e));

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out_dir.join("recompiled.rs"), &out.source).unwrap();
    std::fs::write(out_dir.join("recomp-report.txt"), &out.report).unwrap();
}
