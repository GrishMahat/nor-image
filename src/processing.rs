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

//! Image processing module for handling parallel processing, streaming, and caching operations.
//!
//! This module provides functionality for:
//! - Parallel chunk-based image processing
//! - Streaming large image files
//! - Image caching with LRU policy
//! - Optimized image writing
//! - Parallel compression/decompression

use std::path::Path;
use std::fs::File;
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::sync::Arc;
use rayon::prelude::*;
use lru::LruCache;
use std::sync::Mutex;
use std::num::NonZeroUsize;
use crossbeam_channel::{bounded, Sender, Receiver};
use bytes::{BytesMut, BufMut};
use std::error::Error as StdError;

use crate::format::{CustomImage, CompressionType, FormatError};

/// Default chunk size for parallel processing (1MB)
pub const CHUNK_SIZE: usize = 1024 * 1024;

/// Default number of images to keep in cache
const DEFAULT_CACHE_SIZE: usize = 10;

lazy_static::lazy_static! {
    /// Global LRU cache for storing processed images
    pub static ref IMAGE_CACHE: Mutex<LruCache<String, Arc<CustomImage>>> = 
        Mutex::new(LruCache::new(NonZeroUsize::new(DEFAULT_CACHE_SIZE).unwrap()));
}

/// Errors that can occur during image processing operations
#[derive(Debug)]
pub enum ProcessingError {
    /// Input/output errors
    IoError(io::Error),
    /// Image format related errors
    FormatError(FormatError),
}

impl std::fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingError::IoError(e) => write!(f, "IO error: {}", e),
            ProcessingError::FormatError(e) => write!(f, "Format error: {:?}", e),
        }
    }
}

impl StdError for ProcessingError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ProcessingError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for ProcessingError {
    fn from(err: io::Error) -> Self {
        ProcessingError::IoError(err)
    }
}

impl From<FormatError> for ProcessingError {
    fn from(err: FormatError) -> Self {
        ProcessingError::FormatError(err)
    }
}

/// Processes image data in parallel using fixed-size chunks
///
/// # Arguments
///
/// * `data` - Raw image data to process
/// * `chunk_size` - Size of chunks for parallel processing
///
/// # Returns
///
/// Processed image data as a vector of bytes
pub fn process_parallel(data: &[u8], chunk_size: usize) -> Vec<u8> {
    data.par_chunks(chunk_size)
        .flat_map(|chunk| chunk.to_vec())
        .collect()
}

/// Streaming processor for handling large image files
pub struct StreamingProcessor {
    sender: Sender<Vec<u8>>,
    receiver: Receiver<Vec<u8>>,
    chunk_size: usize,
}

impl StreamingProcessor {
    /// Creates a new StreamingProcessor with specified chunk size
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Size of chunks for streaming processing
    pub fn new(chunk_size: usize) -> Self {
        let (sender, receiver) = bounded(4); // Buffer up to 4 chunks
        StreamingProcessor {
            sender,
            receiver,
            chunk_size,
        }
    }

    /// Processes a stream of image data in chunks
    ///
    /// # Arguments
    ///
    /// * `reader` - Any type implementing Read trait
    ///
    /// # Returns
    ///
    /// Result indicating success or failure of stream processing
    pub fn process_stream<R: Read>(&self, mut reader: R) -> io::Result<()> {
        let mut buffer = vec![0; self.chunk_size];
        
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            
            // Process chunk in parallel
            let processed = process_parallel(&buffer[..n], self.chunk_size);
            self.sender.send(processed).map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "Failed to send processed chunk")
            })?;
        }
        
        Ok(())
    }

    /// Returns an iterator over processed chunks
    pub fn receive_chunks(&self) -> impl Iterator<Item = Vec<u8>> + '_ {
        std::iter::from_fn(move || self.receiver.try_recv().ok())
    }
}

/// Handles loading and caching of images
pub struct CachedImageLoader;

impl CachedImageLoader {
    /// Loads an image from disk with caching
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file
    ///
    /// # Returns
    ///
    /// Arc-wrapped CustomImage or ProcessingError
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Arc<CustomImage>, ProcessingError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        
        // Try to get from cache first
        if let Some(cached) = IMAGE_CACHE.lock().unwrap().get(&path_str) {
            return Ok(Arc::clone(cached));
        }
        
        // Load and process the image
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let image = Self::load_with_streaming(reader)?;
        
        // Cache the result
        let arc_image = Arc::new(image);
        IMAGE_CACHE.lock().unwrap().put(path_str, Arc::clone(&arc_image));
        
        Ok(arc_image)
    }

    /// Internal helper for streaming image loads
    fn load_with_streaming<R: Read>(reader: R) -> Result<CustomImage, ProcessingError> {
        let processor = StreamingProcessor::new(CHUNK_SIZE);
        let mut processed_data = BytesMut::new();
        
        processor.process_stream(reader)?;
        
        for chunk in processor.receive_chunks() {
            processed_data.put_slice(&chunk);
        }
        
        let image = CustomImage::from_bytes(&processed_data)?;
        Ok(image)
    }
}

/// Optimized writer for image data using parallel processing
#[allow(dead_code)]
pub struct OptimizedImageWriter {
    path: Box<Path>,
    chunk_size: usize,
}

impl OptimizedImageWriter {
    /// Creates a new OptimizedImageWriter for the given path
    #[allow(dead_code)]
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        OptimizedImageWriter {
            path: path.as_ref().to_owned().into(),
            chunk_size: CHUNK_SIZE,
        }
    }

    /// Writes an image to disk with parallel processing
    #[allow(dead_code)]
    pub fn write(&self, image: &CustomImage) -> Result<(), ProcessingError> {
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);
        
        let bytes = image.to_bytes()?;
        let processed = process_parallel(&bytes, self.chunk_size);
        
        for chunk in processed.chunks(self.chunk_size) {
            writer.write_all(chunk)?;
        }
        
        writer.flush()?;
        Ok(())
    }
}

/// Handles parallel processing operations on CustomImage
pub struct ParallelImageProcessor;

impl ParallelImageProcessor {
    /// Compresses image data using the specified compression type
    #[allow(dead_code)]
    pub fn compress(image: &mut CustomImage, compression: CompressionType) -> Result<(), FormatError> {
        if image.compression != CompressionType::None {
            return Err(FormatError::CompressionError("Already compressed".to_string()));
        }

        let processed_data = match compression {
            CompressionType::None => image.data.clone(),
            CompressionType::RLE => {
                // Process RLE compression in parallel chunks
                let chunks: Vec<_> = image.data.par_chunks(CHUNK_SIZE)
                    .map(|chunk| CustomImage::compress_rle(chunk))
                    .collect();
                
                let mut result = Vec::new();
                for chunk in chunks {
                    result.extend(chunk);
                }
                result
            }
            CompressionType::Delta => {
                // Delta compression needs sequential processing
                CustomImage::compress_delta(&image.data)
            }
            CompressionType::Lossy => {
                image.compress_lossy(50)?
            }
        };

        image.data = processed_data;
        image.compression = compression;
        Ok(())
    }

    /// Decompresses image data based on its current compression type
    pub fn decompress(image: &mut CustomImage) -> Result<(), FormatError> {
        match image.compression {
            CompressionType::None => Ok(()),
            CompressionType::RLE => {
                let decompressed = CustomImage::decompress_rle(&image.data)?;
                image.data = decompressed;
                image.compression = CompressionType::None;
                Ok(())
            }
            CompressionType::Delta => {
                let decompressed = CustomImage::decompress_delta(&image.data);
                image.data = decompressed;
                image.compression = CompressionType::None;
                Ok(())
            }
            CompressionType::Lossy => {
                let decompressed = CustomImage::decompress_lossy(
                    &image.data,
                    image.width,
                    image.height,
                    image.color_type,
                    50
                )?;
                image.data = decompressed;
                image.compression = CompressionType::None;
                Ok(())
            }
        }
    }
} 