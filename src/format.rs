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

//! Custom Image Format (CIF) implementation.
//!
//! This module provides functionality for working with a custom image format designed
//! for educational purposes. The format supports:
//! 
//! - Multiple color types (Grayscale and RGB)
//! - Various compression methods (None, RLE, Delta, Lossy)
//! - Embedded metadata (stored as JSON)
//! - SHA256 checksum verification for data integrity
//!
//! # File Format Structure
//!
//! The binary format consists of:
//! - Magic number (4 bytes)
//! - Version (1 byte) 
//! - Color type (1 byte)
//! - Width (4 bytes, little-endian)
//! - Height (4 bytes, little-endian)
//! - Compression type (1 byte)
//! - Metadata length (4 bytes, little-endian)
//! - Metadata (JSON string)
//! - Pixel data (uncompressed or compressed bytes)
//! - SHA256 checksum (32 bytes)
//!
//! # Example
//!
//! ```rust
//! use your_crate::format::{CustomImage, ColorType, CompressionType};
//!
//! // Create a dummy 10x10 RGB image with dummy data.
//! let width = 10;
//! let height = 10;
//! let channels = ColorType::Rgb.channels() as usize;
//! let data = vec![128u8; width * height * channels];
//! let image = CustomImage::new(width, height, ColorType::Rgb, data, None, CompressionType::None)?;
//! let bytes = image.to_bytes()?;
//! let decoded_image = CustomImage::from_bytes(&bytes)?;
//! assert_eq!(decoded_image.width, width);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::convert::TryInto;
use std::convert::TryFrom;
use std::time::SystemTime;
use std::error::Error as StdError;
use std::fmt;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Metadata associated with an image.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ImageMetadata {
    /// Creation date of the image (Unix timestamp)
    pub creation_date: u64,
    /// Author/creator of the image
    pub author: Option<String>,
    /// Camera model used
    pub camera_model: Option<String>,
    /// Exposure time in seconds
    pub exposure_time: Option<f32>,
    /// ISO speed
    pub iso: Option<u32>,
    /// F-number
    pub f_number: Option<f32>,
    /// Focal length in mm
    pub focal_length: Option<f32>,
    /// Additional custom metadata as key-value pairs
    pub custom_fields: HashMap<String, String>,
}

impl Default for ImageMetadata {
    fn default() -> Self {
        ImageMetadata {
            creation_date: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            author: None,
            camera_model: None,
            exposure_time: None,
            iso: None,
            f_number: None,
            focal_length: None,
            custom_fields: HashMap::new(),
        }
    }
}

/// Errors that can occur when working with the custom image format.
#[derive(Debug)]
pub enum FormatError {
    /// Data provided is too short to contain a valid header.
    DataTooShort,
    /// The file header did not match the expected magic number.
    InvalidHeader,
    /// The file version is unsupported.
    UnsupportedVersion(u8),
    /// The provided pixel data length does not match the expected size.
    DataLengthMismatch { expected: usize, actual: usize },
    /// Image dimensions are invalid (too large or zero).
    InvalidDimensions { width: u32, height: u32 },
    /// The color type byte in the file is unsupported.
    UnsupportedColorType(u8),
    /// Checksum verification failed.
    ChecksumMismatch,
    /// Error during compression/decompression.
    CompressionError(String),
    /// Error serializing/deserializing metadata.
    MetadataError(String),
}
impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::DataTooShort => write!(f, "Data is too short to contain a valid header"),
            FormatError::InvalidHeader => write!(f, "Invalid file header"),
            FormatError::UnsupportedVersion(v) => write!(f, "Unsupported file version: {}", v),
            FormatError::DataLengthMismatch { expected, actual } => {
                write!(f, "Data length mismatch: expected {}, got {}", expected, actual)
            }
            FormatError::InvalidDimensions { width, height } => {
                write!(f, "Invalid dimensions: {}x{}", width, height)
            }
            FormatError::UnsupportedColorType(ct) => write!(f, "Unsupported color type: {}", ct),
            FormatError::ChecksumMismatch => write!(f, "Checksum verification failed"),
            FormatError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            FormatError::MetadataError(msg) => write!(f, "Metadata error: {}", msg),
        }
    }
}

impl StdError for FormatError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

/// Supported color types for image data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorType {
    /// Single channel grayscale.
    Gray = 0,
    /// Three channel RGB.
    Rgb = 1,
}

impl ColorType {
    /// Returns the number of channels for the color type.
    pub fn channels(&self) -> u32 {
        match self {
            ColorType::Gray => 1,
            ColorType::Rgb => 3,
        }
    }
}

impl TryFrom<u8> for ColorType {
    type Error = FormatError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ColorType::Gray),
            1 => Ok(ColorType::Rgb),
            other => Err(FormatError::UnsupportedColorType(other)),
        }
    }
}

/// Supported compression methods for image data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionType {
    /// No compression.
    None = 0,
    /// Run-length encoding.
    RLE = 1,
    /// Delta encoding.
    Delta = 2,
    /// Lossy compression.
    Lossy = 3,
}

impl TryFrom<u8> for CompressionType {
    type Error = FormatError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::RLE),
            2 => Ok(CompressionType::Delta),
            3 => Ok(CompressionType::Lossy),
            other => Err(FormatError::UnsupportedVersion(other)),
        }
    }
}

/// Represents an image in the Custom Image Format (CIF).
#[derive(Clone, PartialEq, Debug)]
pub struct CustomImage {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// The color type of the image (e.g., grayscale or RGB).
    pub color_type: ColorType,
    /// Raw pixel data as bytes.
    pub data: Vec<u8>,
    /// Image metadata.
    pub metadata: ImageMetadata,
    /// Type of compression used.
    pub compression: CompressionType,
}

/// Constants for the Custom Image Format.
const MAGIC_NUMBER: &[u8] = b"CIMG";
const VERSION: u8 = 2;
const MAX_DIMENSION: u32 = 32_768;

impl CustomImage {
    /// Returns the total number of pixels in the image.
    ///
    /// # Returns
    ///
    /// Returns `Some(count)` if the multiplication of width and height doesn't overflow,
    /// otherwise returns `None`.
    #[allow(dead_code)]
    pub fn pixel_count(&self) -> Option<u32> {
        self.width.checked_mul(self.height)
    }

    /// Validates image dimensions to ensure they are within allowed limits.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if dimensions are valid, otherwise returns an error.
    fn validate_dimensions(width: u32, height: u32) -> Result<(), FormatError> {
        if width == 0 || height == 0 {
            return Err(FormatError::InvalidDimensions { width, height });
        }
        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(FormatError::InvalidDimensions { width, height });
        }
        Ok(())
    }

    /// Creates a new `CustomImage` instance.
    ///
    /// # Arguments
    ///
    /// * `width` - Width of the image in pixels.
    /// * `height` - Height of the image in pixels.
    /// * `color_type` - Color type (Gray or RGB).
    /// * `data` - Raw pixel data.
    /// * `metadata` - Optional image metadata.
    /// * `compression` - Compression method used.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dimensions are invalid (zero or too large).
    /// - Data length doesn't match the expected size for uncompressed images.
    pub fn new(
        width: u32,
        height: u32,
        color_type: ColorType,
        data: Vec<u8>,
        metadata: Option<ImageMetadata>,
        compression: CompressionType,
    ) -> Result<Self, FormatError> {
        Self::validate_dimensions(width, height)?;
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(color_type.channels()))
            .ok_or(FormatError::InvalidDimensions { width, height })? as usize;
        
        if compression == CompressionType::None && data.len() != expected_len {
            return Err(FormatError::DataLengthMismatch {
                expected: expected_len,
                actual: data.len(),
            });
        }

        Ok(CustomImage {
            width,
            height,
            color_type,
            data,
            metadata: metadata.unwrap_or_default(),
            compression,
        })
    }

    /// Compresses data using RLE encoding.
    ///
    /// Run-length encoding compresses sequences of repeated bytes by storing
    /// a count followed by the byte value.
    #[allow(dead_code)]
    pub fn compress_rle(data: &[u8]) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut i = 0;
        
        while i < data.len() {
            let mut count = 1;
            let current = data[i];
            
            while i + count < data.len() && data[i + count] == current && count < 255 {
                count += 1;
            }
            
            compressed.push(count as u8);
            compressed.push(current);
            i += count;
        }
        
        compressed
    }

    /// Decompresses RLE encoded data.
    pub fn decompress_rle(data: &[u8]) -> Result<Vec<u8>, FormatError> {
        let mut decompressed = Vec::new();
        let mut i = 0;
        
        while i < data.len() {
            if i + 1 >= data.len() {
                return Err(FormatError::CompressionError("Invalid RLE data".to_string()));
            }
            
            let count = data[i] as usize;
            let value = data[i + 1];
            decompressed.extend(std::iter::repeat(value).take(count));
            i += 2;
        }
        
        Ok(decompressed)
    }

    /// Compresses data using delta encoding.
    pub fn compress_delta(data: &[u8]) -> Vec<u8> {
        let mut compressed = Vec::with_capacity(data.len());
        if data.is_empty() {
            return compressed;
        }
        
        compressed.push(data[0]);
        for i in 1..data.len() {
            compressed.push(data[i].wrapping_sub(data[i - 1]));
        }
        
        compressed
    }

    /// Decompresses delta encoded data.
    pub fn decompress_delta(data: &[u8]) -> Vec<u8> {
        let mut decompressed = Vec::with_capacity(data.len());
        if data.is_empty() {
            return decompressed;
        }
        
        decompressed.push(data[0]);
        for i in 1..data.len() {
            decompressed.push(decompressed[i - 1].wrapping_add(data[i]));
        }
        
        decompressed
    }

    /// Compresses data using lossy compression.
    ///
    /// The lossy method uses block-based quantization. The quality parameter (1-100)
    /// controls the block size.
    pub fn compress_lossy(&self, quality: u8) -> Result<Vec<u8>, FormatError> {
        let quality = quality.clamp(1, 100) as f32 / 100.0;
        let block_size = if quality < 0.5 { 4 } else { 2 };
        
        let mut compressed = Vec::new();
        match self.color_type {
            ColorType::Gray => {
                // For grayscale, apply block-based quantization.
                for chunk in self.data.chunks(block_size * block_size) {
                    if chunk.len() == block_size * block_size {
                        // Calculate average value for the block.
                        let avg = (chunk.iter().map(|&x| x as u32).sum::<u32>() / 
                                 (block_size * block_size) as u32) as u8;
                        // Store one value for the entire block.
                        compressed.push(avg);
                    } else {
                        // Handle incomplete blocks.
                        compressed.extend_from_slice(chunk);
                    }
                }
            }
            ColorType::Rgb => {
                // For RGB, apply chroma subsampling and block quantization.
                for y in (0..self.height as usize).step_by(block_size) {
                    for x in (0..self.width as usize).step_by(block_size) {
                        let mut r_sum = 0u32;
                        let mut g_sum = 0u32;
                        let mut b_sum = 0u32;
                        let mut count = 0;

                        // Average RGB values for the block.
                        for dy in 0..block_size {
                            for dx in 0..block_size {
                                if y + dy < self.height as usize && x + dx < self.width as usize {
                                    let idx = ((y + dy) * self.width as usize + (x + dx)) * 3;
                                    r_sum += self.data[idx] as u32;
                                    g_sum += self.data[idx + 1] as u32;
                                    b_sum += self.data[idx + 2] as u32;
                                    count += 1;
                                }
                            }
                        }

                        if count > 0 {
                            compressed.push((r_sum / count) as u8);
                            compressed.push((g_sum / count) as u8);
                            compressed.push((b_sum / count) as u8);
                        }
                    }
                }
            }
        }

        Ok(compressed)
    }

    /// Decompresses data that was compressed using lossy compression.
    pub fn decompress_lossy(
        compressed: &[u8],
        width: u32,
        height: u32,
        color_type: ColorType,
        quality: u8,
    ) -> Result<Vec<u8>, FormatError> {
        let quality = quality.clamp(1, 100) as f32 / 100.0;
        let block_size = if quality < 0.5 { 4 } else { 2 };
        
        let mut decompressed = Vec::new();
        match color_type {
            ColorType::Gray => {
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let block_x = (x / block_size) * block_size;
                        let block_y = (y / block_size) * block_size;
                        let block_idx = (block_y * width as usize + block_x) / (block_size * block_size);
                        
                        if block_idx < compressed.len() {
                            decompressed.push(compressed[block_idx]);
                        } else {
                            decompressed.push(0);
                        }
                    }
                }
            }
            ColorType::Rgb => {
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let block_x = (x / block_size) * block_size;
                        let block_y = (y / block_size) * block_size;
                        let block_idx = ((block_y * width as usize + block_x) / (block_size * block_size)) * 3;
                        
                        if block_idx + 2 < compressed.len() {
                            decompressed.push(compressed[block_idx]);     // R
                            decompressed.push(compressed[block_idx + 1]); // G
                            decompressed.push(compressed[block_idx + 2]); // B
                        } else {
                            decompressed.extend_from_slice(&[0, 0, 0]);
                        }
                    }
                }
            }
        }

        Ok(decompressed)
    }

    /// Compresses the image data based on the provided compression type.
    #[allow(dead_code)]
    pub fn compress(&self, compression_type: CompressionType) -> Result<Vec<u8>, FormatError> {
        match compression_type {
            CompressionType::None => Ok(self.data.clone()),
            CompressionType::RLE => Ok(Self::compress_rle(&self.data)),
            CompressionType::Delta => Ok(Self::compress_delta(&self.data)),
            CompressionType::Lossy => self.compress_lossy(50),
        }
    }

    /// Decompresses data based on the provided compression type.
    #[allow(dead_code)]
    pub fn decompress(
        compressed: &[u8],
        width: u32,
        height: u32,
        color_type: ColorType,
        compression_type: CompressionType,
    ) -> Result<Vec<u8>, FormatError> {
        match compression_type {
            CompressionType::None => Ok(compressed.to_vec()),
            CompressionType::RLE => Self::decompress_rle(compressed),
            CompressionType::Delta => Ok(Self::decompress_delta(compressed)),
            CompressionType::Lossy => Self::decompress_lossy(compressed, width, height, color_type, 50),
        }
    }

    /// Serializes the `CustomImage` into a byte vector.
    ///
    /// The format is:
    /// - MAGIC_NUMBER (4 bytes)
    /// - VERSION (1 byte)
    /// - COLOR_TYPE (1 byte)
    /// - Width (4 bytes, little-endian)
    /// - Height (4 bytes, little-endian)
    /// - Compression type (1 byte)
    /// - Metadata length (4 bytes, little-endian)
    /// - Metadata (JSON)
    /// - Image data
    /// - SHA256 checksum (32 bytes)
    pub fn to_bytes(&self) -> Result<Vec<u8>, FormatError> {
        let metadata_json = serde_json::to_string(&self.metadata)
            .unwrap_or_else(|_| "{}".to_string());
        let metadata_bytes = metadata_json.as_bytes();
        
        if metadata_bytes.len() > u32::MAX as usize {
            return Err(FormatError::MetadataError("Metadata size exceeds limit".to_string()));
        }

        let header_len = MAGIC_NUMBER.len() + 1 + 1 + 4 + 4 + 1 + 4 + metadata_bytes.len();
        let total_size = header_len + self.data.len() + 32; // 32 bytes for SHA256 hash
        let mut bytes = Vec::with_capacity(total_size);
        
        // Write header.
        bytes.extend_from_slice(MAGIC_NUMBER);
        bytes.push(VERSION);
        bytes.push(self.color_type as u8);
        bytes.extend_from_slice(&self.width.to_le_bytes());
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.push(self.compression as u8);
        
        // Write metadata.
        bytes.extend_from_slice(&(metadata_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(metadata_bytes);
        
        // Write image data.
        bytes.extend_from_slice(&self.data);
        
        // Calculate and append checksum.
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let checksum: sha2::digest::generic_array::GenericArray<u8, sha2::digest::typenum::UInt<sha2::digest::typenum::UInt<sha2::digest::typenum::UInt<sha2::digest::typenum::UInt<sha2::digest::typenum::UInt<sha2::digest::typenum::UInt<sha2::digest::typenum::UTerm, sha2::digest::consts::B1>, sha2::digest::consts::B0>, sha2::digest::consts::B0>, sha2::digest::consts::B0>, sha2::digest::consts::B0>, sha2::digest::consts::B0>> = hasher.finalize();
        bytes.extend_from_slice(&checksum);
        
        Ok(bytes)
    }

    /// Deserializes a `CustomImage` from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data is too short to contain a valid header.
    /// - The magic number is invalid.
    /// - The version is unsupported.
    /// - The color type is unsupported.
    /// - The pixel data length does not match the expected size.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FormatError> {
        let min_len = MAGIC_NUMBER.len() + 1 + 1 + 4 + 4 + 1 + 4 + 32;
        if bytes.len() < min_len {
            return Err(FormatError::DataTooShort);
        }
        
        // Verify checksum.
        let data_bytes = &bytes[..bytes.len() - 32];
        let file_hash = &bytes[bytes.len() - 32..];
        let mut hasher = Sha256::new();
        hasher.update(data_bytes);
        if &hasher.finalize()[..] != file_hash {
            return Err(FormatError::ChecksumMismatch);
        }
        
        // Read header.
        if &bytes[0..MAGIC_NUMBER.len()] != MAGIC_NUMBER {
            return Err(FormatError::InvalidHeader);
        }
        
        let mut pos = MAGIC_NUMBER.len();
        let file_version = bytes[pos];
        if file_version != VERSION {
            return Err(FormatError::UnsupportedVersion(file_version));
        }
        
        pos += 1;
        let color_type = ColorType::try_from(bytes[pos])?;
        
        pos += 1;
        let width = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let height = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
        pos += 4;
        
        let compression = CompressionType::try_from(bytes[pos])?;
        pos += 1;
        
        // Read metadata.
        if pos + 4 > bytes.len() - 32 {
            return Err(FormatError::DataTooShort);
        }
        let metadata_len = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if pos + metadata_len > bytes.len() - 32 {
            return Err(FormatError::DataTooShort);
        }
        let metadata_json = std::str::from_utf8(&bytes[pos..pos + metadata_len])
            .map_err(|e| FormatError::MetadataError(e.to_string()))?;
        let metadata: ImageMetadata = serde_json::from_str(metadata_json)
            .map_err(|e| FormatError::MetadataError(e.to_string()))?;
        pos += metadata_len;
        
        // Read image data.
        let data = bytes[pos..bytes.len() - 32].to_vec();
        
        Ok(CustomImage {
            width,
            height,
            color_type,
            data,
            metadata,
            compression,
        })
    }
}
