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

//! # Nor-Image CLI
//!
//! A high-performance image processing and conversion tool with an intuitive command-line interface.
//!
//! **Key Features:**
//!
//! - Convert between PNG and the custom `.nor` format with optional processing
//! - Interactive image viewing with real-time adjustments
//! - Comprehensive metadata management
//! - Performance optimizations via caching, streaming, and parallel processing
//!
//! **Usage Examples:**
//!
//!   • `nor-image png-to-custom input.png output.nor`
//!
//!   • `nor-image custom-to-png input.nor output.png`
//!
//!   • `nor-image view image.nor`
//!
//!   • `nor-image info image.nor`
//!
//!   • `nor-image clear-cache`
//!
//! *Tip: Launching `nor-image` without any arguments will start interactive mode.*

use clap::{Parser, Subcommand, ValueEnum};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::converter::{png_to_custom, custom_to_png, ConversionConfig};
use crate::format::{CustomImage, CompressionType};
use crate::viewer::view_custom_image;

mod converter;
mod format;
mod processing;
mod viewer;

use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use env_logger::Builder;
use log::{Level, LevelFilter, Record};

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
///
/// If no subcommand is provided, interactive mode will launch.
#[derive(Parser)]
#[command(
    name = "nor-image",
    author = "Grish <grish@nory.tech>",
    version = "1.0",
    about = "A powerful tool for converting and manipulating images",
    long_about = "Nor-Image CLI\n\
                  \nA high-performance image processing and conversion tool.\n\
                  \nIf no subcommand is provided, interactive mode is launched by default.\n\
                  \nUsage Examples:\n  • nor-image png-to-custom input.png output.nor\n  • nor-image custom-to-png input.nor output.png\n  • nor-image view image.nor\n  • nor-image info image.nor\n  • nor-image clear-cache"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available commands.
#[derive(Subcommand)]
enum Commands {
    /// Convert a PNG file to the custom `.nor` format.
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
    /// Convert a `.nor` file back to PNG format.
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
    /// View a `.nor` image.
    #[command(name = "view", visible_alias = "v")]
    View {
        /// Input .nor file path.
        #[arg(value_name = "IMAGE.nor", help = "Path to .nor image file")]
        input: String,
        /// Use cached version if available.
        #[arg(long, help = "Use cached version for faster loading")]
        use_cache: bool,
    },
    /// Display metadata of a `.nor` image.
    #[command(name = "info", visible_alias = "i")]
    Info {
        /// Input .nor file path.
        #[arg(value_name = "IMAGE.nor", help = "Path to .nor image file")]
        input: String,
    },
    /// Clear the image cache.
    #[command(name = "clear-cache", visible_alias = "cc")]
    ClearCache,
    /// (Optional) Run interactive mode.
    #[command(name = "interactive", visible_alias = "i-mode")]
    Interactive,
}

/// Validates that the provided path has a `.nor` extension.
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

/// Validates that the provided path has a `.png` extension.
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
    println!("\n{}", "Image Information:".bright_cyan().bold());
    println!("{}", "-------------------".bright_cyan());
    println!("{}: {}x{}", "Dimensions".bright_yellow(), image.width, image.height);
    println!("{}: {:?}", "Color Type".bright_yellow(), image.color_type);
    println!("{}: {:?}", "Compression".bright_yellow(), image.compression);
    
    println!("\n{}", "Metadata:".bright_cyan().bold());
    println!("{}: {}", "Creation Date".bright_yellow(), image.metadata.creation_date);
    
    if let Some(ref author) = image.metadata.author {
        println!("{}: {}", "Author".bright_yellow(), author);
    }
    
    if let Some(ref camera) = image.metadata.camera_model {
        println!("{}: {}", "Camera Model".bright_yellow(), camera);
    }
    
    if let Some(exposure) = image.metadata.exposure_time {
        println!("{}: {}s", "Exposure Time".bright_yellow(), exposure);
    }
    
    if let Some(iso) = image.metadata.iso {
        println!("{}: {}", "ISO".bright_yellow(), iso);
    }
    
    if let Some(f_number) = image.metadata.f_number {
        println!("{}: f/{:.1}", "F-Number".bright_yellow(), f_number);
    }
    
    if let Some(focal_length) = image.metadata.focal_length {
        println!("{}: {}mm", "Focal Length".bright_yellow(), focal_length);
    }
    
    if !image.metadata.custom_fields.is_empty() {
        println!("\n{}", "Custom Fields:".bright_cyan().bold());
        for (key, value) in &image.metadata.custom_fields {
            println!("{}: {}", key.bright_yellow(), value);
        }
    }
}

/// Runs the interactive mode using dialoguer prompts.
fn interactive_mode() -> Result<(), Box<dyn Error>> {
    let theme = ColorfulTheme::default();
    
    loop {
        println!("\n{}", "🖼  Nor-Image Interactive Mode".bright_cyan().bold());
        println!("{}\n", "=========================".bright_cyan());
        
        let choices = &[
            "🔄 Convert PNG to custom (.nor)",
            "🔄 Convert custom (.nor) to PNG",
            "👁  View a .nor image",
            "ℹ️  Display image metadata",
            "🗑  Clear cache",
            "❌ Exit",
        ];
        
        let selection = Select::with_theme(&theme)
            .with_prompt("Choose an action")
            .default(0)
            .items(choices)
            .interact()?;

        match selection {
            0 => {
                println!("\n{}", "PNG to NOR Conversion".bright_green().bold());
                let input: String = Input::with_theme(&theme)
                    .with_prompt("Enter input PNG file path")
                    .interact_text()?;
                if let Err(e) = validate_png_extension(&input) {
                    eprintln!("{}: {}", "Error".bright_red().bold(), e);
                    continue;
                }
                let output: String = Input::with_theme(&theme)
                    .with_prompt("Enter output .nor file path")
                    .interact_text()?;
                if let Err(e) = validate_nor_extension(&output) {
                    eprintln!("{}: {}", "Error".bright_red().bold(), e);
                    continue;
                }
                let grayscale: bool = Confirm::with_theme(&theme)
                    .with_prompt("Convert to grayscale?")
                    .default(false)
                    .interact()?;
                let compression_options = &["None", "RLE", "Delta", "Lossy"];
                let comp_index = Select::with_theme(&theme)
                    .with_prompt("Select compression method")
                    .default(0)
                    .items(compression_options)
                    .interact()?;
                let compression = match comp_index {
                    0 => CompressType::None,
                    1 => CompressType::Rle,
                    2 => CompressType::Delta,
                    3 => CompressType::Lossy,
                    _ => CompressType::None,
                };
                let width_input: String = Input::with_theme(&theme)
                    .with_prompt("Enter target width (leave blank for unchanged)")
                    .allow_empty(true)
                    .interact_text()?;
                let width = if width_input.trim().is_empty() {
                    None
                } else {
                    match width_input.trim().parse::<u32>() {
                        Ok(num) => Some(num),
                        Err(_) => {
                            eprintln!("{}: Invalid width", "Error".bright_red().bold());
                            continue;
                        }
                    }
                };
                let height_input: String = Input::with_theme(&theme)
                    .with_prompt("Enter target height (leave blank for unchanged)")
                    .allow_empty(true)
                    .interact_text()?;
                let height = if height_input.trim().is_empty() {
                    None
                } else {
                    match height_input.trim().parse::<u32>() {
                        Ok(num) => Some(num),
                        Err(_) => {
                            eprintln!("{}: Invalid height", "Error".bright_red().bold());
                            continue;
                        }
                    }
                };
                let brightness: i32 = Input::with_theme(&theme)
                    .with_prompt("Enter brightness adjustment (-255 to 255)")
                    .default(0)
                    .interact_text()?;
                let contrast: i32 = Input::with_theme(&theme)
                    .with_prompt("Enter contrast adjustment (-255 to 255)")
                    .default(0)
                    .interact_text()?;
                let no_cache: bool = Confirm::with_theme(&theme)
                    .with_prompt("Disable caching?")
                    .default(false)
                    .interact()?;

                let config = ConversionConfig {
                    resize_width: width,
                    resize_height: height,
                    brightness,
                    contrast,
                    force_grayscale: grayscale,
                    compression: compression.into(),
                    use_cache: !no_cache,
                };

                println!("\n{} {} to {}...", "Converting".bright_yellow(), input, output);
                match png_to_custom(&input, Some(&output), Some(config)) {
                    Ok(_) => println!("{} Successfully converted {} to {}", "✓".bright_green(), input, output),
                    Err(e) => eprintln!("{} {}", "Error:".bright_red().bold(), e),
                }
            }
            1 => {
                println!("\n{}", "NOR to PNG Conversion".bright_green().bold());
                let input: String = Input::with_theme(&theme)
                    .with_prompt("Enter input .nor file path")
                    .interact_text()?;
                if let Err(e) = validate_nor_extension(&input) {
                    eprintln!("{}: {}", "Error".bright_red().bold(), e);
                    continue;
                }
                let output: String = Input::with_theme(&theme)
                    .with_prompt("Enter output PNG file path")
                    .interact_text()?;
                if let Err(e) = validate_png_extension(&output) {
                    eprintln!("{}: {}", "Error".bright_red().bold(), e);
                    continue;
                }
                let width_input: String = Input::with_theme(&theme)
                    .with_prompt("Enter target width (leave blank for unchanged)")
                    .allow_empty(true)
                    .interact_text()?;
                let width = if width_input.trim().is_empty() {
                    None
                } else {
                    match width_input.trim().parse::<u32>() {
                        Ok(num) => Some(num),
                        Err(_) => {
                            eprintln!("{}: Invalid width", "Error".bright_red().bold());
                            continue;
                        }
                    }
                };
                let height_input: String = Input::with_theme(&theme)
                    .with_prompt("Enter target height (leave blank for unchanged)")
                    .allow_empty(true)
                    .interact_text()?;
                let height = if height_input.trim().is_empty() {
                    None
                } else {
                    match height_input.trim().parse::<u32>() {
                        Ok(num) => Some(num),
                        Err(_) => {
                            eprintln!("{}: Invalid height", "Error".bright_red().bold());
                            continue;
                        }
                    }
                };
                let brightness: i32 = Input::with_theme(&theme)
                    .with_prompt("Enter brightness adjustment (-255 to 255)")
                    .default(0)
                    .interact_text()?;
                let contrast: i32 = Input::with_theme(&theme)
                    .with_prompt("Enter contrast adjustment (-255 to 255)")
                    .default(0)
                    .interact_text()?;

                match fs::read(&input) {
                    Ok(bytes) => {
                        match CustomImage::from_bytes(&bytes) {
                            Ok(custom_img) => {
                                let config = ConversionConfig {
                                    resize_width: width,
                                    resize_height: height,
                                    brightness,
                                    contrast,
                                    force_grayscale: false,
                                    compression: CompressionType::None,
                                    use_cache: false,
                                };
                                println!("\n{} {} to {}...", "Converting".bright_yellow(), input, output);
                                match custom_to_png(&custom_img, &output, Some(config)) {
                                    Ok(_) => println!("{} Successfully converted {} to {}", "✓".bright_green(), input, output),
                                    Err(e) => eprintln!("{} {}", "Error:".bright_red().bold(), e),
                                }
                            }
                            Err(e) => eprintln!("{} Reading custom image: {}", "Error:".bright_red().bold(), e),
                        }
                    }
                    Err(e) => eprintln!("{} Reading file: {}", "Error:".bright_red().bold(), e),
                }
            }
            2 => {
                println!("\n{}", "Image Viewer".bright_green().bold());
                let input: String = Input::with_theme(&theme)
                    .with_prompt("Enter .nor image file path")
                    .interact_text()?;
                if let Err(e) = validate_nor_extension(&input) {
                    eprintln!("{}: {}", "Error".bright_red().bold(), e);
                    continue;
                }
                let _use_cache: bool = Confirm::with_theme(&theme)
                    .with_prompt("Use cached version?")
                    .default(false)
                    .interact()?;
                match view_custom_image(&input) {
                    Ok(_) => println!("{} Opened viewer for {}", "✓".bright_green(), input),
                    Err(e) => eprintln!("{} {}", "Error:".bright_red().bold(), e),
                }
            }
            3 => {
                println!("\n{}", "Image Metadata".bright_green().bold());
                let input: String = Input::with_theme(&theme)
                    .with_prompt("Enter .nor image file path")
                    .interact_text()?;
                if let Err(e) = validate_nor_extension(&input) {
                    eprintln!("{}: {}", "Error".bright_red().bold(), e);
                    continue;
                }
                match fs::read(&input) {
                    Ok(bytes) => {
                        match CustomImage::from_bytes(&bytes) {
                            Ok(custom_img) => display_metadata(&custom_img),
                            Err(e) => eprintln!("{} Reading custom image: {}", "Error:".bright_red().bold(), e),
                        }
                    }
                    Err(e) => eprintln!("{} Reading file: {}", "Error:".bright_red().bold(), e),
                }
            }
            4 => {
                println!("\n{}", "Clear Cache".bright_green().bold());
                let confirm = Confirm::with_theme(&theme)
                    .with_prompt("Are you sure you want to clear the image cache?")
                    .default(false)
                    .interact()?;
                if confirm {
                    use crate::processing::IMAGE_CACHE;
                    if let Ok(mut cache) = IMAGE_CACHE.lock() {
                        cache.clear();
                        println!("{} Image cache cleared successfully", "✓".bright_green());
                    } else {
                        eprintln!("{} Failed to clear cache: could not acquire lock", "Error:".bright_red().bold());
                    }
                }
            }
            5 => {
                println!("\n{} Goodbye!", "👋".bright_cyan());
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Main entry point.
fn main() -> Result<(), Box<dyn Error>> {
    // Initialize custom logging with full colored output.
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format(|buf, record: &Record| {
            let ts = buf.timestamp();
            let level = record.level();
            let level_str = match level {
                Level::Error => level.to_string().bright_red().bold(),
                Level::Warn => level.to_string().bright_yellow().bold(),
                Level::Info => level.to_string().bright_green().bold(),
                Level::Debug => level.to_string().bright_blue().bold(),
                Level::Trace => level.to_string().bright_magenta().bold(),
            };
            writeln!(buf, "{} [{}] {}", ts.to_string(), level_str, record.args())
        })
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::PngToCustom {
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
        }) => {
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
            
            println!("\n{}", "Conversion Settings:".bright_cyan().bold());
            println!("  {} {}", "Input:".bright_yellow(), input);
            println!("  {} {}", "Output:".bright_yellow(), output);
            println!("  {} {}", "Grayscale:".bright_yellow(), if grayscale { "yes" } else { "no" });
            println!("  {} {:?}", "Compression:".bright_yellow(), compression);
            if width.is_some() || height.is_some() {
                println!(
                    "  {} {}x{}", 
                    "Resize:".bright_yellow(),
                    width.map_or("unchanged".to_string(), |w| w.to_string()),
                    height.map_or("unchanged".to_string(), |h| h.to_string())
                );
            }
            if brightness != 0 || contrast != 0 {
                println!("  {} brightness={}, contrast={}", "Adjustments:".bright_yellow(), brightness, contrast);
            }
            println!("  {} {}", "Caching:".bright_yellow(), if !no_cache { "enabled" } else { "disabled" });
            
            println!("\n{} Converting...", "⚙️".bright_yellow());
            match png_to_custom(&input, Some(&output), Some(config)) {
                Ok(_) => println!("{} Successfully converted {} to {}", "✓".bright_green(), input, output),
                Err(e) => {
                    eprintln!("{} {}", "Error:".bright_red().bold(), e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
                }
            }
        }
        Some(Commands::CustomToPng {
            input,
            output,
            width,
            height,
            brightness,
            contrast,
            no_streaming: _,
            chunk_size: _,
        }) => {
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
            
            println!("\n{}", "Conversion Settings:".bright_cyan().bold());
            println!("  {} {}", "Input:".bright_yellow(), input);
            println!("  {} {}", "Output:".bright_yellow(), output);
            if width.is_some() || height.is_some() {
                println!(
                    "  {} {}x{}", 
                    "Resize:".bright_yellow(),
                    width.map_or("unchanged".to_string(), |w| w.to_string()),
                    height.map_or("unchanged".to_string(), |h| h.to_string())
                );
            }
            if brightness != 0 || contrast != 0 {
                println!("  {} brightness={}, contrast={}", "Adjustments:".bright_yellow(), brightness, contrast);
            }
            
            println!("\n{} Converting...", "⚙️".bright_yellow());
            match custom_to_png(&custom_img, &output, Some(config)) {
                Ok(_) => println!("{} Successfully converted {} to {}", "✓".bright_green(), input, output),
                Err(e) => {
                    eprintln!("{} {}", "Error:".bright_red().bold(), e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
                }
            }
        }
        Some(Commands::View { input, use_cache: _ }) => {
            validate_nor_extension(&input)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            println!("\n{} Opening viewer...", "👁".bright_yellow());
            view_custom_image(&input)?;
        }
        Some(Commands::Info { input }) => {
            validate_nor_extension(&input)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            let bytes = fs::read(&input)?;
            let custom_img = CustomImage::from_bytes(&bytes)?;
            display_metadata(&custom_img);
        }
        Some(Commands::ClearCache) => {
            use crate::processing::IMAGE_CACHE;
            if let Ok(mut cache) = IMAGE_CACHE.lock() {
                cache.clear();
                println!("{} Image cache cleared successfully", "✓".bright_green());
            } else {
                eprintln!("{} Failed to clear cache: could not acquire lock", "Error:".bright_red().bold());
            }
        }
        _ => {
            interactive_mode()?;
        }
    }
    Ok(())
}
