//! op-engine — native CLI baker.
//!
//! Reads the heightmap + texture PNGs and writes the compact terrain-grid
//! artifact the web app loads (`web/<slug>-terrain.json`). All terrain math
//! lives in `opengine-core`; this binary only does PNG I/O + file writing.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use opengine_core::terrain::{bake, PixelBuf};

#[derive(Parser, Debug)]
#[command(
    name = "op-engine",
    about = "Bake an op-display terrain grid from heightmap/texture PNGs"
)]
struct Args {
    /// Grayscale heightmap PNG (white = high). Omit with --from-color.
    #[arg(long)]
    height: Option<PathBuf>,

    /// Map texture PNG (drives the material grid + the color fallback).
    #[arg(long)]
    texture: Option<PathBuf>,

    /// Output artifact path.
    #[arg(long, default_value = "../web/endless-shore-terrain.json")]
    out: PathBuf,

    /// Plane subdivisions in X (grid width = seg_x + 1).
    #[arg(long, default_value_t = 320)]
    seg_x: usize,

    /// Plane subdivisions in Y (grid height = seg_y + 1).
    #[arg(long, default_value_t = 277)]
    seg_y: usize,

    /// Default displacement multiplier (applied in JS; tunable without re-baking).
    #[arg(long, default_value_t = 0.8)]
    relief: f32,

    /// Ignore the heightmap and infer relief from texture colors.
    #[arg(long)]
    from_color: bool,

    /// Artifact slug (defaults to the output file stem).
    #[arg(long)]
    slug: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // --- decode texture (RGB) ---
    let tex_img = match &args.texture {
        Some(p) => Some(
            image::open(p)
                .with_context(|| format!("opening texture {}", p.display()))?
                .to_rgb8(),
        ),
        None => None,
    };
    let tex_buf = tex_img.as_ref().map(|img| PixelBuf {
        data: img.as_raw(),
        w: img.width() as usize,
        h: img.height() as usize,
        channels: 3,
    });

    // --- decode heightmap (luminance) unless --from-color ---
    let height_img = match (&args.height, args.from_color) {
        (Some(p), false) => Some(
            image::open(p)
                .with_context(|| format!("opening heightmap {}", p.display()))?
                .to_luma8(),
        ),
        _ => None,
    };
    let height_buf = height_img.as_ref().map(|img| PixelBuf {
        data: img.as_raw(),
        w: img.width() as usize,
        h: img.height() as usize,
        channels: 1,
    });

    let slug = args.slug.clone().unwrap_or_else(|| {
        args.out
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("terrain")
            .trim_end_matches("-terrain")
            .to_string()
    });
    let source = args
        .height
        .as_ref()
        .or(args.texture.as_ref())
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("none")
        .to_string();

    let artifact = bake(
        height_buf.as_ref(),
        tex_buf.as_ref(),
        args.seg_x,
        args.seg_y,
        args.relief,
        &slug,
        &source,
    );

    let json = serde_json::to_string(&artifact).context("serializing artifact")?;
    std::fs::write(&args.out, &json)
        .with_context(|| format!("writing {}", args.out.display()))?;

    let g = &artifact.grid;
    println!(
        "baked {} ({}x{} grid, {} cells, relief {}) from {} -> {} ({} KB)",
        slug,
        g.w,
        g.h,
        g.w * g.h,
        artifact.meta.relief,
        source,
        args.out.display(),
        json.len() / 1024,
    );
    Ok(())
}
