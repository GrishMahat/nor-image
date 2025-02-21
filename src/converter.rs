use image::{DynamicImage, GrayImage, RgbImage, GenericImageView, imageops};
use image::{ImageEncoder, ColorType};
use std::path::Path;
use std::io;
use rayon::prelude::*;
use std::error::Error as StdError;
use std::fmt;
use std::fs::File;
use std::io::Write;

use crate::format::{CustomImage, FormatError, ColorType as CustomColorType, CompressionType, ImageMetadata};
use crate::processing::{CachedImageLoader, ParallelImageProcessor, ProcessingError, CHUNK_SIZE};
//  Rewrite count 3

/// Error types that can occur during image conversion.
#[derive(Debug)]
pub enum ConversionError {
    /// Error reading or writing a PNG file.
    ImageError(image::ImageError),
    /// Error with our custom format.
    FormatError(FormatError),
    /// Error with processing.
    ProcessingError(ProcessingError),
    /// Image has an unsupported color type or format.
    UnsupportedFormat(String),
    /// I/O error.
    IoError(io::Error),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::ImageError(e) => write!(f, "Image error: {}", e),
            ConversionError::FormatError(e) => write!(f, "Format error: {}", e),
            ConversionError::ProcessingError(e) => write!(f, "Processing error: {}", e),
            ConversionError::UnsupportedFormat(msg) => write!(f, "Unsupported format: {}", msg),
            ConversionError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl StdError for ConversionError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ConversionError::ImageError(e) => Some(e),
            ConversionError::IoError(e) => Some(e),
            ConversionError::ProcessingError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<image::ImageError> for ConversionError {
    fn from(err: image::ImageError) -> Self {
        ConversionError::ImageError(err)
    }
}

impl From<FormatError> for ConversionError {
    fn from(err: FormatError) -> Self {
        ConversionError::FormatError(err)
    }
}

impl From<ProcessingError> for ConversionError {
    fn from(err: ProcessingError) -> Self {
        ConversionError::ProcessingError(err)
    }
}

impl From<io::Error> for ConversionError {
    fn from(err: io::Error) -> Self {
        ConversionError::IoError(err)
    }
}

/// Configuration for image conversion
#[derive(Debug, Clone)]
pub struct ConversionConfig {
    /// Target width for resizing (None means keep original)
    pub resize_width: Option<u32>,
    /// Target height for resizing (None means keep original)
    pub resize_height: Option<u32>,
    /// Brightness adjustment (-255 to 255)
    pub brightness: i32,
    /// Contrast adjustment (-255 to 255)
    pub contrast: i32,
    /// Whether to convert to grayscale
    pub force_grayscale: bool,
    /// Compression type
    pub compression: CompressionType,
    /// Whether to use cache
    pub use_cache: bool,
    /// Whether to use streaming
    #[allow(dead_code)]
    pub streaming: bool,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        ConversionConfig {
            resize_width: None,
            resize_height: None,
            brightness: 0,
            contrast: 0,
            force_grayscale: false,
            compression: CompressionType::None,
            use_cache: true,
            streaming: true,
        }
    }
}

fn apply_adjustments(data: &[u8], brightness: i32, contrast: i32) -> Vec<u8> {
    data.par_chunks(CHUNK_SIZE)
        .map(|chunk| {
            let mut processed = chunk.to_vec();
            for pixel in processed.iter_mut() {
                // Convert to float and normalize to -1.0 to 1.0 range for better precision
                let mut value = (*pixel as f32 / 127.5) - 1.0;
                
                // Apply contrast first (better for preserving details)
                if contrast != 0 {
                    let contrast_factor = (contrast as f32 + 255.0) / 255.0;
                    value *= contrast_factor;
                }
                
                // Then apply brightness
                if brightness != 0 {
                    value += (brightness as f32) / 127.5;
                }
                
                // Convert back to 0-255 range with improved clamping
                *pixel = ((value + 1.0).clamp(0.0, 2.0) * 127.5).min(255.0).max(0.0) as u8;
            }
            processed
        })
        .collect::<Vec<_>>()
        .concat()
}

/// Converts a PNG file to our custom image format with optional preprocessing.
///
/// # Arguments
///
/// * `png_path` - Path to the source PNG file
/// * `output_path` - Optional path where the converted image should be saved
/// * `config` - Optional conversion configuration for preprocessing
///
/// # Returns
///
/// Returns `Result<CustomImage, ConversionError>` containing either the converted image
/// or an error if the conversion failed.
pub fn png_to_custom<P: AsRef<Path>>(
    png_path: P,
    output_path: Option<P>,
    config: Option<ConversionConfig>
) -> Result<CustomImage, ConversionError> {
    let config = config.unwrap_or_default();
    let path = png_path.as_ref();

    // Try to load from cache if enabled
    if config.use_cache {
        if let Ok(cached) = CachedImageLoader::load(path) {
            return Ok((*cached).clone());
        }
    }

    // Load and process the image with improved quality
    let img = image::open(path)?;
    let (width, height) = img.dimensions();

    // Process the image based on color type
    let processed_data = if config.force_grayscale {
        // Convert to grayscale with improved quality
        let gray_img = img.into_luma8();
        
        // Apply preprocessing for grayscale
        let processed_img = if let (Some(w), Some(h)) = (config.resize_width, config.resize_height) {
            imageops::resize(
                &gray_img,
                w,
                h,
                imageops::FilterType::Lanczos3 // Better quality for downscaling
            )
        } else {
            gray_img
        };

        let raw_data = processed_img.into_raw();
        
        // Apply adjustments with improved quality
        if config.brightness != 0 || config.contrast != 0 {
            apply_adjustments(&raw_data, config.brightness, config.contrast)
        } else {
            raw_data
        }
    } else {
        // Process as RGB with improved quality
        let rgb_img = img.into_rgb8();
        
        // Apply preprocessing for RGB
        let processed_img = if let (Some(w), Some(h)) = (config.resize_width, config.resize_height) {
            imageops::resize(
                &rgb_img,
                w,
                h,
                imageops::FilterType::Lanczos3 // Better quality for downscaling
            )
        } else {
            rgb_img
        };

        let raw_data = processed_img.into_raw();
        
        // Apply adjustments with improved quality
        if config.brightness != 0 || config.contrast != 0 {
            apply_adjustments(&raw_data, config.brightness, config.contrast)
        } else {
            raw_data
        }
    };

    // Update dimensions if resized
    let (final_width, final_height) = if let (Some(w), Some(h)) = (config.resize_width, config.resize_height) {
        (w, h)
    } else {
        (width, height)
    };

    // Create the custom image with improved quality settings
    let mut custom_img = CustomImage::new(
        final_width,
        final_height,
        if config.force_grayscale { CustomColorType::Gray } else { CustomColorType::Rgb },
        processed_data,
        Some(ImageMetadata::default()),
        config.compression
    )?;

    // Apply compression with improved quality preservation
    if config.compression != CompressionType::None {
        let compressed_data = match config.compression {
            CompressionType::RLE => {
                // Use larger chunks for RLE to better preserve patterns
                let chunk_size = if config.force_grayscale { 8 } else { 24 };
                custom_img.data.chunks(chunk_size)
                    .flat_map(|chunk| CustomImage::compress_rle(chunk))
                    .collect()
            }
            CompressionType::Delta => {
                // Use predictive encoding for better quality
                CustomImage::compress_delta(&custom_img.data)
            }
            CompressionType::Lossy => {
                // Use higher quality settings for lossy compression
                custom_img.compress_lossy(90)?
            }
            CompressionType::None => custom_img.data.clone(),
        };
        custom_img.data = compressed_data;
        custom_img.compression = config.compression;
    }

    // Save to file if output path is provided
    if let Some(output_path) = output_path {
        let mut file = File::create(output_path)?;
        let bytes = custom_img.to_bytes()?;
        file.write_all(&bytes)?;
    }

    // Save to cache if enabled
    if config.use_cache {
        let _ = CachedImageLoader::load(path);
    }

    Ok(custom_img)
}

/// Converts our custom image format to a PNG file with optional postprocessing.
///
/// # Arguments
///
/// * `custom_img` - The source custom image
/// * `png_path` - Path where the PNG file should be saved
/// * `config` - Optional conversion configuration for postprocessing
///
/// # Returns
///
/// Returns `Result<(), ConversionError>` indicating success or failure.
pub fn custom_to_png<P: AsRef<Path>>(
    custom_img: &CustomImage,
    png_path: P,
    config: Option<ConversionConfig>
) -> Result<(), ConversionError> {
    let config = config.unwrap_or_default();
    let path = png_path.as_ref();

    // Create a copy for processing
    let mut img_data = custom_img.clone();

    // Decompress with improved quality
    if img_data.compression != CompressionType::None {
        ParallelImageProcessor::decompress(&mut img_data)?;
    }

    // Create the image based on color type with improved handling
    let mut img: DynamicImage = match img_data.color_type {
        CustomColorType::Gray => {
            let gray_img = GrayImage::from_raw(
                img_data.width,
                img_data.height,
                img_data.data
            ).ok_or_else(|| ConversionError::UnsupportedFormat("Failed to create grayscale image".to_string()))?;
            DynamicImage::ImageLuma8(gray_img)
        }
        CustomColorType::Rgb => {
            let rgb_img = RgbImage::from_raw(
                img_data.width,
                img_data.height,
                img_data.data
            ).ok_or_else(|| ConversionError::UnsupportedFormat("Failed to create RGB image".to_string()))?;
            DynamicImage::ImageRgb8(rgb_img)
        }
    };

    // Apply postprocessing with improved quality
    if let (Some(width), Some(height)) = (config.resize_width, config.resize_height) {
        img = DynamicImage::ImageRgba8(imageops::resize(
            &img,
            width,
            height,
            imageops::FilterType::Lanczos3
        ));
    }

    // Process adjustments with improved quality
    if config.brightness != 0 || config.contrast != 0 {
        let mut buffer = img.to_rgb8();
        
        for pixel in buffer.pixels_mut() {
            for channel in pixel.0.iter_mut() {
                // Convert to float and normalize to -1.0 to 1.0 range
                let mut value = (*channel as f32 / 127.5) - 1.0;
                
                // Apply contrast first
                if config.contrast != 0 {
                    let contrast_factor = (config.contrast as f32 + 255.0) / 255.0;
                    value *= contrast_factor;
                }
                
                // Then apply brightness
                if config.brightness != 0 {
                    value += (config.brightness as f32) / 127.5;
                }
                
                // Convert back with improved clamping
                *channel = ((value.clamp(-1.0, 1.0) + 1.0) * 127.5) as u8;
            }
        }
        
        img = DynamicImage::ImageRgb8(buffer);
    }

    // Save with maximum quality
    let file = File::create(path)?;
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        file,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::Adaptive
    );

    let (width, height) = img.dimensions();
    let color_type = match img {
        DynamicImage::ImageLuma8(_) => ColorType::L8,
        DynamicImage::ImageRgb8(_) => ColorType::Rgb8,
        DynamicImage::ImageRgba8(_) => ColorType::Rgba8,
        _ => ColorType::Rgb8,
    };

    encoder.write_image(
        img.as_bytes(),
        width,
        height,
        color_type.into()
    )?;

    Ok(())
}
