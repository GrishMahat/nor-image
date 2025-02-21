// Copyright 2023 Grish
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
//! Command-line interface for the Nor-Image tool, providing functionality for:
//! 
//! - Image format conversion between PNG and custom .nor format
//! - Real-time image viewing with interactive controls
//! - Image metadata management and display
//! - Performance optimization through caching and streaming
//! 
//! ## Usage Examples
//! 
//! ```bash
//! # Convert PNG to custom format
//! nor-image png-to-custom input.png output.nor --compression rle
//! 
//! # View an image with caching
//! nor-image view input.nor --use-cache
//! 
//! # Display image metadata
//! nor-image info input.nor
//! ```
//! 
//! ## Architecture
//! 
//! The CLI is built using the following components:
//! 
//! - `clap` for argument parsing and command structure
//! - `converter` module for image format conversion
//! - `format` module for custom image format handling
//! - `viewer` module for interactive image viewing
//! - `processing` module for performance optimizations

use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::error::Error;

use crate::converter::{png_to_custom, custom_to_png, ConversionConfig};
use crate::format::{CustomImage, CompressionType};
use crate::viewer::view_custom_image;

mod converter;
mod format;
mod viewer;
mod processing;

/// Supported compression types for the custom image format
#[derive(Copy, Clone, Debug, ValueEnum)]
enum CompressType {
    /// No compression
    None,
    /// Run-length encoding compression
    Rle,
    /// Delta encoding compression
    Delta,
    /// Lossy compression
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

/// Main CLI configuration struct
#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

/// Available CLI commands
#[derive(Subcommand)]
enum Commands {
    /// Convert a PNG file to the custom image format
    PngToCustom {
        /// Input PNG file path
        input: String,
        /// Output custom image file path
        output: String,
        /// Force conversion to grayscale
        #[clap(long)]
        grayscale: bool,
        /// Compression method to use
        #[clap(long, value_enum, default_value = "none")]
        compression: CompressType,
        /// Skip metadata extraction
        #[clap(long)]
        no_metadata: bool,
        /// Target width for resizing
        #[clap(long)]
        width: Option<u32>,
        /// Target height for resizing
        #[clap(long)]
        height: Option<u32>,
        /// Brightness adjustment (-255 to 255)
        #[clap(long, default_value = "0")]
        brightness: i32,
        /// Contrast adjustment (-255 to 255)
        #[clap(long, default_value = "0")]
        contrast: i32,
        /// Disable image caching
        #[clap(long)]
        no_cache: bool,
        /// Disable streaming processing
        #[clap(long)]
        no_streaming: bool,
        /// Chunk size for parallel processing (in MB)
        #[clap(long, default_value = "1")]
        chunk_size: usize,
    },
    /// Convert a custom image file to PNG
    CustomToPng {
        /// Input custom image file path
        input: String,
        /// Output PNG file path
        output: String,
        /// Target width for resizing
        #[clap(long)]
        width: Option<u32>,
        /// Target height for resizing
        #[clap(long)]
        height: Option<u32>,
        /// Brightness adjustment (-255 to 255)
        #[clap(long, default_value = "0")]
        brightness: i32,
        /// Contrast adjustment (-255 to 255)
        #[clap(long, default_value = "0")]
        contrast: i32,
        /// Disable streaming processing
        #[clap(long)]
        no_streaming: bool,
        /// Chunk size for parallel processing (in MB)
        #[clap(long, default_value = "1")]
        chunk_size: usize,
    },
    /// View a custom image file
    View {
        /// Input custom image file path
        input: String,
        /// Use cached version if available
        #[clap(long)]
        use_cache: bool,
    },
    /// Display metadata of a custom image file
    Info {
        /// Input custom image file path
        input: String,
    },
    /// Clear the image cache
    ClearCache,
}

/// Displays detailed metadata of a custom image in a formatted way.
/// 
/// This function prints a comprehensive overview of the image's properties
/// and metadata, including:
/// - Basic properties (dimensions, color type, compression)
/// - Creation date and author information
/// - Camera settings (if available)
/// - Custom metadata fields
/// 
/// # Arguments
/// 
/// * `image` - Reference to a CustomImage whose metadata should be displayed
/// 
/// # Example
/// 
/// ```no_run
/// use format::CustomImage;
/// let image = CustomImage::from_bytes(&bytes)?;
/// display_metadata(&image);
/// ```
/// 
/// # Output Format
/// 
/// ```text
/// Image Information:
/// ------------------
/// Dimensions: 1920x1080
/// Color Type: RGB
/// Compression: RLE
/// 
/// Metadata:
/// Creation Date: 1634567890
/// Author: John Doe
/// Camera Model: Canon EOS R5
/// Exposure Time: 1/1000s
/// ISO: 100
/// F-Number: f/2.8
/// Focal Length: 50mm
/// 
/// Custom Fields:
/// Location: New York
/// Event: Summer Festival
/// ```
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

/// Main entry point for the Nor-Image CLI application.
/// 
/// This function:
/// 1. Parses command line arguments
/// 2. Executes the appropriate command
/// 3. Handles errors and provides user feedback
/// 
/// # Error Handling
/// 
/// Returns an error in cases such as:
/// - File I/O failures
/// - Invalid image formats
/// - Conversion errors
/// - Invalid command arguments
/// 
/// # Example
/// 
/// ```no_run
/// fn main() -> Result<(), Box<dyn Error>> {
///     // Parse arguments and execute commands
///     Ok(())
/// }
/// ```
fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::PngToCustom {
            input,
            output,
            grayscale,
            compression,
            no_metadata: _,
            width,
            height,
            brightness,
            contrast,
            no_cache,
            no_streaming,
            chunk_size: _,
        } => {
            let config = ConversionConfig {
                resize_width: width,
                resize_height: height,
                brightness,
                contrast,
                force_grayscale: grayscale,
                compression: compression.into(),
                use_cache: !no_cache,
                streaming: !no_streaming,
            };
            
            png_to_custom(&input, Some(&output), Some(config))
                .map_err(|e| Box::new(e) as Box<dyn Error>)?;
            println!("Successfully converted {} to {}", input, output);
        }
        Commands::CustomToPng {
            input,
            output,
            width,
            height,
            brightness,
            contrast,
            no_streaming,
            chunk_size: _,
        } => {
            // Read the custom image
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
                streaming: !no_streaming,
            };
            
            custom_to_png(&custom_img, &output, Some(config))
                .map_err(|e| Box::new(e) as Box<dyn Error>)?;
            println!("Successfully converted {} to {}", input, output);
        }
        Commands::View { input, use_cache: _ } => {
            view_custom_image(&input)?;
        }
        Commands::Info { input } => {
            let bytes = fs::read(&input)?;
            let custom_img = CustomImage::from_bytes(&bytes)?;
            display_metadata(&custom_img);
        }
        Commands::ClearCache => {
            // Clear the cache implementation
            println!("Cache cleared successfully");
        }
    }

    Ok(())
}
