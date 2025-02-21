// Copyright 2025 Grish
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Most of the documentation is by AI, I am too lazy to write it myself.
//! # Nor-Image CLI
//!
//! A high-performance image processing and conversion tool with an intuitive command-line interface.
//!
//! ## Key Features
//!
//! - Convert between PNG and .nor formats with optional processing
//! - Interactive image viewing with real-time adjustments
//! - Comprehensive metadata management
//! - Performance optimizations through caching, streaming, and parallel processing
//!
//! ## Basic Usage
//!
//! ```bash
//! nor-image png-to-custom input.png output.nor    # Convert PNG to custom format
//! nor-image custom-to-png input.nor output.png      # Convert custom format to PNG
//! nor-image view image.nor                          # View a .nor image
//! nor-image info image.nor                          # Display image metadata
//! nor-image clear-cache                             # Clear image cache
//! ```
//!
//! For more details, run `nor-image --help`.

use clap::{Parser, Subcommand, ValueEnum};
use std::error::Error;
use std::fs;
use std::path::Path;

use crate::converter::{png_to_custom, custom_to_png, ConversionConfig};
use crate::format::{CustomImage, CompressionType};
use crate::viewer::view_custom_image;

mod converter;
mod format;
mod viewer;
mod processing;

/// Supported compression types for the custom image format.
#[derive(Copy, Clone, Debug, ValueEnum)]
enum CompressType {
    /// No compression – Maximum quality, largest file size.
    None,
    /// Run-length encoding – Best for images with large uniform areas.
    Rle,
    /// Delta encoding – Efficient for photographs with gradual color changes.
    Delta,
    /// Lossy compression – Smallest file size, configurable quality.
    Lossy,
}

impl From<CompressType> for CompressionType {
    fn from(ct: CompressType) -> Self {
        match ct {
            CompressType::None => CompressionType::None,
            CompressType::Rle => CompressionType::RLE,
            CompressType::Delta => CompressionType::Delta,
            CompressType::Lossy => CompressionType::Lossy,
        }
    }
}

/// Nor-Image: High-performance image processing and conversion tool.
#[derive(Parser)]
#[command(name = "nor-image")]
#[command(author = "Grish <grish@nory.tech>")]
#[command(version = "1.0")]
#[command(about = "A powerful image processing tool for converting and manipulating images", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available commands.
#[derive(Subcommand)]
enum Commands {
    /// Convert a PNG file to the custom .nor format.
    #[command(name = "png-to-custom", visible_alias = "p2n")]
    PngToCustom {
        /// Input PNG file path (must have .png extension).
        #[arg(value_name = "INPUT.png")]
        input: String,
        /// Output .nor file path (must have .nor extension).
        #[arg(value_name = "OUTPUT.nor")]
        output: String,
        /// Convert image to grayscale.
        #[arg(long, help = "Convert to grayscale (reduces file size)")]
        grayscale: bool,
        /// Compression method.
        #[arg(long, value_enum, default_value = "none", help = "Compression method")]
        compression: CompressType,
        /// Target width for resizing.
        #[arg(long, value_name = "PIXELS", help = "Resize to specified width")]
        width: Option<u32>,
        /// Target height for resizing.
        #[arg(long, value_name = "PIXELS", help = "Resize to specified height")]
        height: Option<u32>,
        /// Brightness adjustment (-255 to 255).
        #[arg(long, default_value = "0", value_name = "VALUE", help = "Adjust brightness (-255 to 255)")]
        brightness: i32,
        /// Contrast adjustment (-255 to 255).
        #[arg(long, default_value = "0", value_name = "VALUE", help = "Adjust contrast (-255 to 255)")]
        contrast: i32,
        /// Disable image caching.
        #[arg(long, help = "Disable caching for faster processing")]
        no_cache: bool,
        /// Disable streaming processing.
        #[arg(long, help = "Disable streaming (uses more memory)")]
        no_streaming: bool,
        /// Chunk size for parallel processing (in MB).
        #[arg(long, default_value = "1", value_name = "MB", help = "Chunk size for parallel processing (MB)")]
        chunk_size: usize,
    },
    /// Convert a .nor file back to PNG format.
    #[command(name = "custom-to-png", visible_alias = "n2p")]
    CustomToPng {
        /// Input .nor file path (must have .nor extension).
        #[arg(value_name = "input.nor")]
        input: String,
        /// Output PNG file path (must have .png extension).
        #[arg(value_name = "output.png")]
        output: String,
        /// Target width for resizing.
        #[arg(long, value_name = "PIXELS", help = "Resize to specified width")]
        width: Option<u32>,
        /// Target height for resizing.
        #[arg(long, value_name = "PIXELS", help = "Resize to specified height")]
        height: Option<u32>,
        /// Brightness adjustment (-255 to 255).
        #[arg(long, default_value = "0", value_name = "VALUE", help = "Adjust brightness (-255 to 255)")]
        brightness: i32,
        /// Contrast adjustment (-255 to 255).
        #[arg(long, default_value = "0", value_name = "VALUE", help = "Adjust contrast (-255 to 255)")]
        contrast: i32,
        /// Disable streaming processing.
        #[arg(long, help = "Disable streaming (uses more memory)")]
        no_streaming: bool,
        /// Chunk size for parallel processing (in MB).
        #[arg(long, default_value = "1", value_name = "MB", help = "Chunk size for parallel processing (MB)")]
        chunk_size: usize,
    },
    /// View a .nor image interactively.
    #[command(name = "view", visible_alias = "v")]
    View {
        /// Input .nor file path.
        #[arg(value_name = "IMAGE.nor", help = "Path to .nor image file")]
        input: String,
        /// Use cached version if available.
        #[arg(long, help = "Use cached version for faster loading")]
        use_cache: bool,
    },
    /// Display metadata of a .nor image.
    #[command(name = "info", visible_alias = "i")]
    Info {
        /// Input .nor file path.
        #[arg(value_name = "IMAGE.nor", help = "Path to .nor image file")]
        input: String,
    },
    /// Clear the image cache.
    #[command(name = "clear-cache", visible_alias = "cc")]
    ClearCache,
}

/// Validates that the provided path has a .nor extension.
fn validate_nor_extension(path: &str) -> Result<(), String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    if ext == "nor" {
        Ok(())
    } else {
        Err(format!("Invalid file extension. Expected .nor, got: {}", path))
    }
}

/// Validates that the provided path has a .png extension.
fn validate_png_extension(path: &str) -> Result<(), String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    if ext == "png" {
        Ok(())
    } else {
        Err(format!("Invalid file extension. Expected .png, got: {}", path))
    }
}

/// Displays metadata of a custom image in a formatted way.
fn display_metadata(image: &CustomImage) {
    println!("\nImage Information:");
    println!("------------------");
    println!("Dimensions: {}x{}", image.width, image.height);
    println!("Color Type: {:?}", image.color_type);
    println!("Compression: {:?}", image.compression);
    println!("\nMetadata:");
    println!("Creation Date: {}", image.metadata.creation_date);
    
    if let Some(ref author) = image.metadata.author {
        println!("Author: {}", author);
    }
    
    if let Some(ref camera) = image.metadata.camera_model {
        println!("Camera Model: {}", camera);
    }
    
    if let Some(exposure) = image.metadata.exposure_time {
        println!("Exposure Time: {}s", exposure);
    }
    
    if let Some(iso) = image.metadata.iso {
        println!("ISO: {}", iso);
    }
    
    if let Some(f_number) = image.metadata.f_number {
        println!("F-Number: f/{:.1}", f_number);
    }
    
    if let Some(focal_length) = image.metadata.focal_length {
        println!("Focal Length: {}mm", focal_length);
    }
    
    if !image.metadata.custom_fields.is_empty() {
        println!("\nCustom Fields:");
        for (key, value) in &image.metadata.custom_fields {
            println!("{}: {}", key, value);
        }
    }
}

/// Main entry point.
fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging.
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::PngToCustom {
            input,
            output,
            grayscale,
            compression,
            width,
            height,
            brightness,
            contrast,
            no_cache,
            no_streaming: _,
            chunk_size: _,
        } => {
            // Validate file extensions.
            validate_png_extension(&input)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            validate_nor_extension(&output)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

            let config = ConversionConfig {
                resize_width: width,
                resize_height: height,
                brightness,
                contrast,
                force_grayscale: grayscale,
                compression: compression.into(),
                use_cache: !no_cache,
            };
            
            println!("Converting {} to {} with settings:", input, output);
            println!("  - Grayscale: {}", if grayscale { "yes" } else { "no" });
            println!("  - Compression: {:?}", compression);
            if width.is_some() || height.is_some() {
                println!("  - Resize: {}x{}", 
                    width.map_or("unchanged".to_string(), |w| w.to_string()),
                    height.map_or("unchanged".to_string(), |h| h.to_string()));
            }
            if brightness != 0 || contrast != 0 {
                println!("  - Adjustments: brightness={}, contrast={}", brightness, contrast);
            }
            println!("  - Caching: {}", if !no_cache { "enabled" } else { "disabled" });
            
            match png_to_custom(&input, Some(&output), Some(config)) {
                Ok(_) => println!("✓ Successfully converted {} to {}", input, output),
                Err(e) => {
                    eprintln!("Error during conversion:");
                    eprintln!("  {}", e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
                }
            }
        }
        Commands::CustomToPng {
            input,
            output,
            width,
            height,
            brightness,
            contrast,
            no_streaming: _,
            chunk_size: _,
        } => {
            validate_nor_extension(&input)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            validate_png_extension(&output)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            
            let bytes = fs::read(&input)?;
            let custom_img = CustomImage::from_bytes(&bytes)?;
            
            let config = ConversionConfig {
                resize_width: width,
                resize_height: height,
                brightness,
                contrast,
                force_grayscale: false,
                compression: CompressionType::None,
                use_cache: false,
            };
            
            println!("Converting {} to {} with settings:", input, output);
            if width.is_some() || height.is_some() {
                println!("  - Resize: {}x{}", 
                    width.map_or("unchanged".to_string(), |w| w.to_string()),
                    height.map_or("unchanged".to_string(), |h| h.to_string()));
            }
            if brightness != 0 || contrast != 0 {
                println!("  - Adjustments: brightness={}, contrast={}", brightness, contrast);
            }
            
            match custom_to_png(&custom_img, &output, Some(config)) {
                Ok(_) => println!("✓ Successfully converted {} to {}", input, output),
                Err(e) => {
                    eprintln!("Error during conversion:");
                    eprintln!("  {}", e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
                }
            }
        }
        Commands::View { input, use_cache: _ } => {
            validate_nor_extension(&input)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            view_custom_image(&input)?;
        }
        Commands::Info { input } => {
            validate_nor_extension(&input)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            let bytes = fs::read(&input)?;
            let custom_img = CustomImage::from_bytes(&bytes)?;
            display_metadata(&custom_img);
        }
        Commands::ClearCache => {
            use crate::processing::IMAGE_CACHE;
            if let Ok(mut cache) = IMAGE_CACHE.lock() {
                cache.clear();
                println!("Image cache cleared successfully");
            } else {
                eprintln!("Failed to clear cache: could not acquire lock");
            }
        }
    }
    Ok(())
}
