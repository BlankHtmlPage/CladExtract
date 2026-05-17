//! Image preview loading, KTX decoding, and texture cache management.
//!
//! Handles PNG, WebP, KTX1 (BC1/BC3/BC7), and KTX2 (Zstandard supercompressed)
//! texture formats. Supports BC1 (DXT1), BC3 (DXT3/DXT5), and BC7 compression.
//! Enforces a configurable maximum image dimension to prevent excessive RAM
//! consumption and provides cache eviction for GPU texture handles.

use crate::config;
use crate::gui;
use crate::logic;
use egui::TextureHandle;
use std::num::NonZero;
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;

static ASSETS_LOADING: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

const KTX1_MAGIC: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x31, 0x31, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

const KTX2_MAGIC: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

// GL compressed format constants (used by KTX1)
const GL_COMPRESSED_RGB_S3TC_DXT1_EXT: u32 = 0x83F0;
const GL_COMPRESSED_RGBA_S3TC_DXT1_EXT: u32 = 0x83F1;
const GL_COMPRESSED_RGBA_S3TC_DXT3_EXT: u32 = 0x83F2;
const GL_COMPRESSED_RGBA_S3TC_DXT5_EXT: u32 = 0x83F3;
const GL_COMPRESSED_SRGB_S3TC_DXT1_EXT: u32 = 0x8C4C;
const GL_COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT: u32 = 0x8C4D;
const GL_COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT: u32 = 0x8C4E;
const GL_COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT: u32 = 0x8C4F;
const GL_COMPRESSED_RGBA_BPTC_UNORM_EXT: u32 = 0x8E8C;
const GL_COMPRESSED_SRGB_ALPHA_BPTC_UNORM_EXT: u32 = 0x8E8D;
// ETC2 format constants (used by Roblox KTX1 textures)
const GL_COMPRESSED_RGB8_ETC2: u32 = 0x9274;
const GL_COMPRESSED_RGBA8_ETC2_EAC: u32 = 0x9278;

// Vulkan format constants (used by KTX2)
const VK_FORMAT_BC1_RGB_UNORM_BLOCK: u32 = 131;
const VK_FORMAT_BC1_RGB_SRGB_BLOCK: u32 = 132;
const VK_FORMAT_BC1_RGBA_UNORM_BLOCK: u32 = 133;
const VK_FORMAT_BC3_UNORM_BLOCK: u32 = 137;
const VK_FORMAT_BC3_SRGB_BLOCK: u32 = 138;
const VK_FORMAT_BC7_UNORM_BLOCK: u32 = 145;
const VK_FORMAT_BC7_SRGB_BLOCK: u32 = 146;

/// Convert a single linear (UNORM) byte value to sRGB.
/// egui operates in gamma/sRGB space, so linear decompressed values
/// must be converted before display to avoid yellow color shift.
fn linear_to_srgb(linear: u8) -> u8 {
    let l = linear as f32 / 255.0;
    let s = if l <= 0.0031308 {
        12.92 * l
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Apply linear-to-sRGB conversion to RGB channels of an RGBA8 pixel buffer.
/// Alpha is left unchanged (alpha is always linear).
fn apply_srgb_correction(rgba8: &mut [u8]) {
    for chunk in rgba8.chunks_exact_mut(4) {
        chunk[0] = linear_to_srgb(chunk[0]);
        chunk[1] = linear_to_srgb(chunk[1]);
        chunk[2] = linear_to_srgb(chunk[2]);
    }
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Returns `(width, height)` from image data without full decoding.
/// Supports PNG, WebP, KTX1, and KTX2 headers.
pub fn get_image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 12 {
        return None;
    }

    // PNG
    if data.len() >= 24 && &data[1..4] == b"PNG" {
        return read_u32_be(data, 16).zip(read_u32_be(data, 20));
    }

    // WebP lossless (VP8L)
    if data.len() >= 30 && &data[12..16] == b"VP8L" {
        let bits = u32::from_le_bytes([data[21], data[22], data[23], data[24]]);
        let w = (bits & 0x3FFF) + 1;
        let h = ((bits >> 14) & 0x3FFF) + 1;
        return Some((w, h));
    }

    // WebP lossy (VP8)
    if data.len() >= 30 && &data[12..16] == b"VP8 " {
        if data.len() >= 33 {
            let w = u16::from_le_bytes([data[26], data[27]]) as u32;
            let h = u16::from_le_bytes([data[29], data[30]]) as u32;
            if w > 0 && h > 0 {
                return Some((w, h));
            }
        }
    }

    // KTX2
    if data.len() >= 80 && data.starts_with(&KTX2_MAGIC) {
        if let Ok(reader) = ktx2::Reader::new(data) {
            let header = reader.header();
            return Some((header.pixel_width, header.pixel_height));
        }
    }

    // KTX1
    if data.len() >= 64 && data.starts_with(&KTX1_MAGIC) {
        return read_u32_le(data, 36).zip(read_u32_le(data, 40));
    }

    None
}

/// Checks if image dimensions exceed the configured maximum.
pub fn exceeds_max_dimensions(data: &[u8]) -> Option<String> {
    let max_dim = config::get_config_u64("max_preview_dimension").unwrap_or(4096) as u32;
    if let Some((w, h)) = get_image_dimensions(data) {
        if w > max_dim || h > max_dim {
            return Some(format!("image dimensions {w}x{h} exceed limit {max_dim}"));
        }
    }
    None
}

/// Decode KTX1 or KTX2 texture data to RGBA8 pixels.
/// Returns `(width, height, rgba8_pixels)`.
fn decode_ktx(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if data.len() >= 12 && data.starts_with(&KTX2_MAGIC) {
        decode_ktx2(data)
    } else if data.len() >= 12 && data.starts_with(&KTX1_MAGIC) {
        decode_ktx1(data)
    } else {
        Err("not a KTX file".to_string())
    }
}

fn decode_ktx1(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if data.len() < 64 {
        return Err("KTX1 data too short".to_string());
    }

    let gl_internal_format = read_u32_le(data, 28).ok_or("invalid KTX1 header")?;
    let width = read_u32_le(data, 36).ok_or("invalid KTX1 header")? as usize;
    let height = read_u32_le(data, 40).ok_or("invalid KTX1 header")? as usize;
    let bytes_of_kv = read_u32_le(data, 60).ok_or("invalid KTX1 header")? as usize;

    if width == 0 || height == 0 {
        return Err("KTX1: zero dimensions".to_string());
    }

    // Skip 64-byte header + key/value data
    let mip_offset = 64 + bytes_of_kv;
    if data.len() < mip_offset + 4 {
        return Err("KTX1: no mip data".to_string());
    }

    let image_size = u32::from_le_bytes([
        data[mip_offset],
        data[mip_offset + 1],
        data[mip_offset + 2],
        data[mip_offset + 3],
    ]) as usize;
    let pixel_data = data
        .get(mip_offset + 4..mip_offset + 4 + image_size)
        .ok_or("KTX1: truncated mip level")?;

    let is_srgb;
    let mut rgba8 = match gl_internal_format {
        // BC1 (DXT1) — UNORM (linear data, needs sRGB correction)
        GL_COMPRESSED_RGB_S3TC_DXT1_EXT | GL_COMPRESSED_RGBA_S3TC_DXT1_EXT => {
            is_srgb = false;
            bc1_decode_rgba8(pixel_data, width, height)?.2
        }
        // BC1 (DXT1) — sRGB (already in display color space)
        GL_COMPRESSED_SRGB_S3TC_DXT1_EXT | GL_COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT => {
            is_srgb = true;
            bc1_decode_rgba8(pixel_data, width, height)?.2
        }
        // BC3 (DXT3/DXT5) — UNORM (linear)
        GL_COMPRESSED_RGBA_S3TC_DXT3_EXT | GL_COMPRESSED_RGBA_S3TC_DXT5_EXT => {
            is_srgb = false;
            bc3_decode_rgba8(pixel_data, width, height)?.2
        }
        // BC3 (DXT3/DXT5) — sRGB
        GL_COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT | GL_COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT => {
            is_srgb = true;
            bc3_decode_rgba8(pixel_data, width, height)?.2
        }
        // BC7 — UNORM (linear)
        GL_COMPRESSED_RGBA_BPTC_UNORM_EXT => {
            is_srgb = false;
            bc7_decode_rgba8(pixel_data, width, height)?.2
        }
        // BC7 — sRGB
        GL_COMPRESSED_SRGB_ALPHA_BPTC_UNORM_EXT => {
            is_srgb = true;
            bc7_decode_rgba8(pixel_data, width, height)?.2
        }
        // ETC2 RGB — UNORM (linear, used by Roblox)
        GL_COMPRESSED_RGB8_ETC2 => {
            is_srgb = false;
            etc2_rgb_decode_rgba8(pixel_data, width, height)?.2
        }
        // ETC2 RGBA8 (EAC) — UNORM (linear, used by Roblox)
        GL_COMPRESSED_RGBA8_ETC2_EAC => {
            is_srgb = false;
            etc2_rgba8_decode_rgba8(pixel_data, width, height)?.2
        }
        _ => {
            return Err(format!(
                "KTX1: unsupported format 0x{gl_internal_format:04X}"
            ));
        }
    };

    if !is_srgb {
        apply_srgb_correction(&mut rgba8);
    }

    Ok((width as u32, height as u32, rgba8))
}

fn decode_ktx2(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let reader = ktx2::Reader::new(data).map_err(|e| format!("KTX2 parse error: {e}"))?;
    let header = reader.header();
    let width = header.pixel_width as usize;
    let height = header.pixel_height as usize;

    if width == 0 || height == 0 {
        return Err("KTX2: zero dimensions".to_string());
    }

    let levels: Vec<ktx2::Level> = reader.levels().collect();
    let first_level = levels.first().ok_or("KTX2: no mip levels")?;
    let raw_level_data = first_level.data;

    let level_bytes = match header.supercompression_scheme {
        None => raw_level_data.to_vec(),
        Some(scheme) if scheme == ktx2::SupercompressionScheme::Zstandard => {
            zstd::decode_all(raw_level_data)
                .map_err(|e| format!("KTX2 zstd decompress: {e}"))?
        }
        Some(scheme) => {
            return Err(format!("KTX2: unsupported supercompression {scheme:?}"));
        }
    };

    let format_value = header
        .format
        .as_ref()
        .map(|f| f.value())
        .unwrap_or(0);
    let is_srgb;
    let mut rgba8 = match format_value {
        // BC1 — UNORM (linear)
        VK_FORMAT_BC1_RGB_UNORM_BLOCK | VK_FORMAT_BC1_RGBA_UNORM_BLOCK => {
            is_srgb = false;
            bc1_decode_rgba8(&level_bytes, width, height)?.2
        }
        // BC1 — sRGB
        VK_FORMAT_BC1_RGB_SRGB_BLOCK => {
            is_srgb = true;
            bc1_decode_rgba8(&level_bytes, width, height)?.2
        }
        // BC3 — UNORM (linear)
        VK_FORMAT_BC3_UNORM_BLOCK => {
            is_srgb = false;
            bc3_decode_rgba8(&level_bytes, width, height)?.2
        }
        // BC3 — sRGB
        VK_FORMAT_BC3_SRGB_BLOCK => {
            is_srgb = true;
            bc3_decode_rgba8(&level_bytes, width, height)?.2
        }
        // BC7 — UNORM (linear)
        VK_FORMAT_BC7_UNORM_BLOCK => {
            is_srgb = false;
            bc7_decode_rgba8(&level_bytes, width, height)?.2
        }
        // BC7 — sRGB
        VK_FORMAT_BC7_SRGB_BLOCK => {
            is_srgb = true;
            bc7_decode_rgba8(&level_bytes, width, height)?.2
        }
        _ => {
            return Err(format!(
                "KTX2: unsupported VkFormat 0x{format_value:04X}"
            ));
        }
    };

    if !is_srgb {
        apply_srgb_correction(&mut rgba8);
    }

    Ok((width as u32, height as u32, rgba8))
}

/// Decode BC1 (DXT1) compressed data to RGBA8 pixels.
/// Each 8-byte block decodes to a 4×4 pixel tile.
fn bc1_decode_rgba8(
    compressed: &[u8],
    width: usize,
    height: usize,
) -> Result<(u32, u32, Vec<u8>), String> {
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;
    let block_count = blocks_x * blocks_y;
    let expected_size = block_count * 8;

    if compressed.len() < expected_size {
        return Err(format!(
            "BC1: need {expected_size} bytes, got {}",
            compressed.len()
        ));
    }

    let stride = width * 4;
    let mut rgba8 = vec![0u8; height * stride];
    let mut tile = [0u8; 4 * 4 * 4];

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_offset = (by * blocks_x + bx) * 8;
            let block_data = &compressed[block_offset..block_offset + 8];

            bcdec_rs::bc1(block_data, &mut tile, 16);

            // Copy tile into the output image
            for ty in 0..4 {
                let py = by * 4 + ty;
                if py >= height {
                    break;
                }
                let row_offset = py * stride + bx * 4 * 4;
                let tile_row_offset = ty * 16;
                let copy_len = (4 * 4).min(stride.saturating_sub(bx * 4 * 4));
                rgba8[row_offset..row_offset + copy_len]
                    .copy_from_slice(&tile[tile_row_offset..tile_row_offset + copy_len]);
            }
        }
    }

    Ok((width as u32, height as u32, rgba8))
}

fn bc7_decode_rgba8(
    compressed: &[u8],
    width: usize,
    height: usize,
) -> Result<(u32, u32, Vec<u8>), String> {
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;
    let block_count = blocks_x * blocks_y;
    let expected_size = block_count * 16;

    if compressed.len() < expected_size {
        return Err(format!(
            "BC7: need {expected_size} bytes, got {}",
            compressed.len()
        ));
    }

    let stride = width * 4;
    let mut rgba8 = vec![0u8; height * stride];
    let mut tile = [0u8; 4 * 4 * 4];

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_offset = (by * blocks_x + bx) * 16;
            let block_data = &compressed[block_offset..block_offset + 16];

            bcdec_rs::bc7(block_data, &mut tile, 16);

            for ty in 0..4 {
                let py = by * 4 + ty;
                if py >= height {
                    break;
                }
                let row_offset = py * stride + bx * 4 * 4;
                let tile_row_offset = ty * 16;
                let copy_len = (4 * 4).min(stride.saturating_sub(bx * 4 * 4));
                rgba8[row_offset..row_offset + copy_len]
                    .copy_from_slice(&tile[tile_row_offset..tile_row_offset + copy_len]);
            }
        }
    }

    Ok((width as u32, height as u32, rgba8))
}

fn bc3_decode_rgba8(
    compressed: &[u8],
    width: usize,
    height: usize,
) -> Result<(u32, u32, Vec<u8>), String> {
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;
    let block_count = blocks_x * blocks_y;
    let expected_size = block_count * 16;

    if compressed.len() < expected_size {
        return Err(format!(
            "BC3 (DXT5): need {expected_size} bytes, got {}",
            compressed.len()
        ));
    }

    let stride = width * 4;
    let mut rgba8 = vec![0u8; height * stride];
    let mut tile = [0u8; 4 * 4 * 4];

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_offset = (by * blocks_x + bx) * 16;
            let block_data = &compressed[block_offset..block_offset + 16];

            bcdec_rs::bc3(block_data, &mut tile, 16);

            for ty in 0..4 {
                let py = by * 4 + ty;
                if py >= height {
                    break;
                }
                let row_offset = py * stride + bx * 4 * 4;
                let tile_row_offset = ty * 16;
                let copy_len = (4 * 4).min(stride.saturating_sub(bx * 4 * 4));
                rgba8[row_offset..row_offset + copy_len]
                    .copy_from_slice(&tile[tile_row_offset..tile_row_offset + copy_len]);
            }
        }
    }

    Ok((width as u32, height as u32, rgba8))
}

/// Decode ETC2 RGB compressed data to RGBA8 pixels.
/// texture2ddecoder outputs ARGB u32 pixels (le bytes: [b, g, r, a]).
fn etc2_rgb_decode_rgba8(
    compressed: &[u8],
    width: usize,
    height: usize,
) -> Result<(u32, u32, Vec<u8>), String> {
    let mut pixels = vec![0u32; width * height];
    texture2ddecoder::decode_etc2_rgb(compressed, width, height, &mut pixels)
        .map_err(|e| format!("ETC2 RGB decode: {e}"))?;

    let mut rgba8 = Vec::with_capacity(pixels.len() * 4);
    for pixel in &pixels {
        let bytes = pixel.to_le_bytes(); // [b, g, r, a]
        rgba8.push(bytes[2]); // R
        rgba8.push(bytes[1]); // G
        rgba8.push(bytes[0]); // B
        rgba8.push(bytes[3]); // A
    }

    Ok((width as u32, height as u32, rgba8))
}

/// Decode ETC2 RGBA8 (EAC) compressed data to RGBA8 pixels.
fn etc2_rgba8_decode_rgba8(
    compressed: &[u8],
    width: usize,
    height: usize,
) -> Result<(u32, u32, Vec<u8>), String> {
    let mut pixels = vec![0u32; width * height];
    texture2ddecoder::decode_etc2_rgba8(compressed, width, height, &mut pixels)
        .map_err(|e| format!("ETC2 RGBA8 decode: {e}"))?;

    let mut rgba8 = Vec::with_capacity(pixels.len() * 4);
    for pixel in &pixels {
        let bytes = pixel.to_le_bytes(); // [b, g, r, a]
        rgba8.push(bytes[2]); // R
        rgba8.push(bytes[1]); // G
        rgba8.push(bytes[0]); // B
        rgba8.push(bytes[3]); // A
    }

    Ok((width as u32, height as u32, rgba8))
}

/// Load an image from raw bytes, storing the result in the global texture cache.
pub fn load_image_from_bytes(
    id: &str,
    data: &[u8],
    ctx: egui::Context,
) -> Result<TextureHandle, String> {
    let images = { gui::IMAGES.lock().unwrap().clone() };
    if let Some(texture) = images.get(id) {
        return Ok(texture.clone());
    }

    let (width, height, rgba8) = if data.len() >= 12
        && (data.starts_with(&KTX2_MAGIC) || data.starts_with(&KTX1_MAGIC))
    {
        decode_ktx(data)?
    } else {
        let img = image::load_from_memory(data).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let w = rgba.width();
        let h = rgba.height();
        (w, h, rgba.into_raw())
    };

    let texture = ctx.load_texture(
        id,
        egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            &rgba8,
        ),
        Default::default(),
    );

    let mut images = gui::IMAGES.lock().unwrap();
    images.insert(id.to_string(), texture.clone());
    Ok(texture)
}

/// Async-wrapper for loading an asset image in the background.
/// Spawns a thread to extract image bytes and load them into the texture cache.
/// Returns `None` immediately; the texture will be available on the next frame.
pub fn load_asset_image(asset: logic::AssetInfo, ctx: egui::Context) -> Option<TextureHandle> {
    {
        let images = gui::IMAGES.lock().unwrap();
        if let Some(texture) = images.get(&asset.name) {
            return Some(texture.clone());
        }
    }

    // Concurrency throttle
    {
        let assets_loading = ASSETS_LOADING.lock().unwrap();
        if assets_loading.contains(&asset.name)
            || assets_loading.len()
                >= thread::available_parallelism()
                    .unwrap_or(NonZero::new(2).unwrap())
                    .into()
        {
            return None;
        }
    }

    let asset_name = asset.name.clone();
    thread::spawn(move || {
        {
            let mut assets_loading = ASSETS_LOADING.lock().unwrap();
            assets_loading.push(asset_name.clone());
        }

        match logic::extract_asset_to_bytes(asset.clone()) {
            Ok(bytes) => {
                if let Some(reason) = exceeds_max_dimensions(&bytes) {
                    log_warn!("Skipping {asset_name}: {reason}");
                    thread::sleep(Duration::from_millis(1000));
                    let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                    assets_loading.retain(|x| x != &asset_name);
                    return;
                }

                match load_image_from_bytes(&asset_name, &bytes, ctx) {
                    Ok(_) => {
                        let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                        assets_loading.retain(|x| x != &asset_name);
                    }
                    Err(e) => {
                        log_warn!(
                            "Failed to load {asset_name} as image, cooldown for 1000 ms ({e})"
                        );
                        thread::sleep(Duration::from_millis(1000));
                        let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                        assets_loading.retain(|x| x != &asset_name);
                    }
                }
            }
            Err(e) => {
                log_error!("Unable to read file {asset_name}, 1000 ms cooldown: {e}");
                thread::sleep(Duration::from_millis(1000));
                let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                assets_loading.retain(|x| x != &asset_name);
            }
        }
    });

    None
}

/// Releases all cached GPU textures and clears the image cache.
pub fn clear_all_images(ctx: &egui::Context) {
    let mut images = gui::IMAGES.lock().unwrap();
    for id in images.keys() {
        ctx.forget_image(id);
    }
    images.clear();
}