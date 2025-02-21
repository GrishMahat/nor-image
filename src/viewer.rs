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


use minifb::{Window, WindowOptions, Key, Scale, KeyRepeat};
use crate::format::{CustomImage, ColorType};
use std::fs;
use std::error::Error;

pub struct ImageViewer {
    window: Window,
    buffer: Vec<u32>,
    original_buffer: Vec<u32>,
    width: usize,
    height: usize,
    zoom: f32,
    brightness: i32,
    contrast: i32,
    color_type: ColorType,
    last_window_size: (usize, usize),
    edge_detection: bool,
}

impl ImageViewer {
    pub fn new(custom_image: CustomImage) -> Result<Self, Box<dyn Error>> {
        let width = custom_image.width as usize;
        let height = custom_image.height as usize;
        
        let mut window = Window::new(
            &format!("Image Viewer ({}x{}) - Press H for help", width, height),
            width,
            height,
            WindowOptions {
                scale: Scale::X1,
                resize: true,
                ..WindowOptions::default()
            },
        ).map_err(|e| format!("Failed to create window: {}", e))?;

        // Set a reasonable FPS limit
        window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

        let mut original_buffer: Vec<u32> = vec![0; width * height];
        
        // Convert image data to RGB format for display
        match custom_image.color_type {
            ColorType::Gray => {
                for i in 0..original_buffer.len() {
                    let pixel = custom_image.data[i] as u32;
                    original_buffer[i] = (pixel << 16) | (pixel << 8) | pixel;
                }
            }
            ColorType::Rgb => {
                for (i, chunk) in custom_image.data.chunks_exact(3).enumerate() {
                    let r = chunk[0] as u32;
                    let g = chunk[1] as u32;
                    let b = chunk[2] as u32;
                    original_buffer[i] = (r << 16) | (g << 8) | b;
                }
            }
        }

        let mut viewer = ImageViewer {
            window,
            buffer: original_buffer.clone(),
            original_buffer,
            width,
            height,
            zoom: 1.0,
            brightness: 0,
            contrast: 0,
            color_type: custom_image.color_type,
            last_window_size: (width, height),
            edge_detection: false,
        };

        // Initialize the display by updating the window buffer
        viewer.update_window_buffer()?;

        Ok(viewer)
    }

    fn apply_adjustments(&mut self) {
        // Convert to grayscale if edge detection is enabled
        if self.edge_detection {
            self.apply_edge_detection();
            return;
        }

        for i in 0..self.buffer.len() {
            let pixel = self.original_buffer[i];
            let r = ((pixel >> 16) & 0xFF) as i32;
            let g = ((pixel >> 8) & 0xFF) as i32;
            let b = (pixel & 0xFF) as i32;

            // Apply brightness
            let r = (r + self.brightness).clamp(0, 255);
            let g = (g + self.brightness).clamp(0, 255);
            let b = (b + self.brightness).clamp(0, 255);

            // Apply contrast
            let contrast = self.contrast.clamp(-255, 255);
            let contrast_factor = (259.0 * (contrast as f32 + 255.0)) / (255.0 * (259.0 - contrast as f32));
            let r = ((contrast_factor * (r as f32 - 128.0) + 128.0).clamp(0.0, 255.0)) as u32;
            let g = ((contrast_factor * (g as f32 - 128.0) + 128.0).clamp(0.0, 255.0)) as u32;
            let b = ((contrast_factor * (b as f32 - 128.0) + 128.0).clamp(0.0, 255.0)) as u32;

            self.buffer[i] = (r << 16) | (g << 8) | b;
        }
    }

    fn apply_edge_detection(&mut self) {
        // Create grayscale version of the image
        let mut grayscale: Vec<u8> = vec![0; self.width * self.height];
        for i in 0..self.original_buffer.len() {
            let pixel = self.original_buffer[i];
            let r = ((pixel >> 16) & 0xFF) as u32;
            let g = ((pixel >> 8) & 0xFF) as u32;
            let b = (pixel & 0xFF) as u32;
            // Convert to grayscale using luminance formula
            grayscale[i] = ((0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8)
                .saturating_add(self.brightness as u8);
        }

        // Sobel operators
        let sobel_x: [[i32; 3]; 3] = [
            [-1, 0, 1],
            [-2, 0, 2],
            [-1, 0, 1]
        ];
        let sobel_y: [[i32; 3]; 3] = [
            [-1, -2, -1],
            [0, 0, 0],
            [1, 2, 1]
        ];

        let mut edges = vec![0u32; self.width * self.height];

        // Apply Sobel operator
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let mut gx = 0i32;
                let mut gy = 0i32;

                // Apply convolution
                for ky in 0..3 {
                    for kx in 0..3 {
                        let pixel = grayscale[(y + ky - 1) * self.width + (x + kx - 1)] as i32;
                        gx += pixel * sobel_x[ky][kx];
                        gy += pixel * sobel_y[ky][kx];
                    }
                }

                // Calculate gradient magnitude
                let magnitude = ((gx * gx + gy * gy) as f32).sqrt() as u32;
                let magnitude = magnitude.clamp(0, 255);

                // Apply threshold for better edge visibility
                let threshold = 50u32;
                let edge_value = if magnitude > threshold { 255 } else { 0 };

                // Store edge pixel
                edges[y * self.width + x] = (edge_value << 16) | (edge_value << 8) | edge_value;
            }
        }

        self.buffer = edges;
    }

    fn show_help(&self) {
        println!("\nImage Viewer Controls:");
        println!("----------------------");
        println!("ESC - Exit");
        println!("H   - Show this help");
        println!("+ / - - Zoom in/out");
        println!("↑ / ↓ - Adjust brightness");
        println!("← / → - Adjust contrast");
        println!("E   - Toggle edge detection");
        println!("R   - Reset adjustments");
        println!("I   - Show image info");
    }

    fn show_info(&self) {
        println!("\nImage Information:");
        println!("------------------");
        println!("Dimensions: {}x{}", self.width, self.height);
        println!("Color Type: {:?}", self.color_type);
        println!("Current zoom: {:.1}x", self.zoom);
        println!("Brightness: {}", self.brightness);
        println!("Contrast: {}", self.contrast);
        println!("Edge Detection: {}", if self.edge_detection { "On" } else { "Off" });
        let (win_width, win_height) = self.window.get_size();
        println!("Window size: {}x{}", win_width, win_height);
    }

    fn update_window_buffer(&mut self) -> Result<(), Box<dyn Error>> {
        let (win_width, win_height) = self.window.get_size();
        
        // Calculate dimensions that maintain aspect ratio
        let aspect_ratio = self.width as f32 / self.height as f32;
        let win_aspect_ratio = win_width as f32 / win_height as f32;
        
        let (scaled_width, scaled_height) = if win_aspect_ratio > aspect_ratio {
            let height = win_height as f32;
            let width = height * aspect_ratio;
            ((width * self.zoom) as usize, (height * self.zoom) as usize)
        } else {
            let width = win_width as f32;
            let height = width / aspect_ratio;
            ((width * self.zoom) as usize, (height * self.zoom) as usize)
        };
        
        // Create a resized buffer
        let mut resized_buffer = vec![0; scaled_width * scaled_height];
        
        // Bilinear interpolation for smoother scaling
        for y in 0..scaled_height {
            for x in 0..scaled_width {
                let src_x = (x as f32 / self.zoom) * (self.width as f32 / scaled_width as f32);
                let src_y = (y as f32 / self.zoom) * (self.height as f32 / scaled_height as f32);
                
                // Get the four surrounding pixels
                let x0 = src_x.floor() as usize;
                let x1 = (x0 + 1).min(self.width - 1);
                let y0 = src_y.floor() as usize;
                let y1 = (y0 + 1).min(self.height - 1);
                
                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;
                
                // Get the four surrounding pixels
                let p00 = self.buffer[y0 * self.width + x0];
                let p10 = self.buffer[y0 * self.width + x1];
                let p01 = self.buffer[y1 * self.width + x0];
                let p11 = self.buffer[y1 * self.width + x1];
                
                // Extract RGB components
                let (r00, g00, b00) = ((p00 >> 16) & 0xFF, (p00 >> 8) & 0xFF, p00 & 0xFF);
                let (r10, g10, b10) = ((p10 >> 16) & 0xFF, (p10 >> 8) & 0xFF, p10 & 0xFF);
                let (r01, g01, b01) = ((p01 >> 16) & 0xFF, (p01 >> 8) & 0xFF, p01 & 0xFF);
                let (r11, g11, b11) = ((p11 >> 16) & 0xFF, (p11 >> 8) & 0xFF, p11 & 0xFF);
                
                // Bilinear interpolation for each color channel
                let r = (r00 as f32 * (1.0 - fx) * (1.0 - fy) +
                       r10 as f32 * fx * (1.0 - fy) +
                       r01 as f32 * (1.0 - fx) * fy +
                       r11 as f32 * fx * fy) as u32;
                
                let g = (g00 as f32 * (1.0 - fx) * (1.0 - fy) +
                       g10 as f32 * fx * (1.0 - fy) +
                       g01 as f32 * (1.0 - fx) * fy +
                       g11 as f32 * fx * fy) as u32;
                
                let b = (b00 as f32 * (1.0 - fx) * (1.0 - fy) +
                       b10 as f32 * fx * (1.0 - fy) +
                       b01 as f32 * (1.0 - fx) * fy +
                       b11 as f32 * fx * fy) as u32;
                
                resized_buffer[y * scaled_width + x] = (r << 16) | (g << 8) | b;
            }
        }
        
        // Update the window buffer with the new size
        self.window.update_with_buffer(&resized_buffer, scaled_width, scaled_height)
            .map_err(|e| format!("Failed to update window buffer: {}", e))?;
        
        self.last_window_size = (win_width, win_height);
        
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.show_help();

        while self.window.is_open() && !self.window.is_key_down(Key::Escape) {
            // Check for window resize
            let current_size = self.window.get_size();
            if current_size != self.last_window_size {
                self.last_window_size = current_size;
                self.update_window_buffer()?;
            }


            // Handle zoom
            if self.window.is_key_pressed(Key::Equal, KeyRepeat::No) {
                self.zoom = (self.zoom * 1.2).min(10.0);
                self.update_window_buffer()?;
            }
            if self.window.is_key_pressed(Key::Minus, KeyRepeat::No) {
                self.zoom = (self.zoom / 1.2).max(0.1);
                self.update_window_buffer()?;
            }

            // Handle brightness
            if self.window.is_key_down(Key::Up) {
                self.brightness = (self.brightness + 1).clamp(-255, 255);
                self.apply_adjustments();
                self.update_window_buffer()?;
            }
            if self.window.is_key_down(Key::Down) {
                self.brightness = (self.brightness - 1).clamp(-255, 255);
                self.apply_adjustments();
                self.update_window_buffer()?;
            }

            // Handle contrast
            if self.window.is_key_down(Key::Right) {
                self.contrast = (self.contrast + 1).clamp(-255, 255);
                self.apply_adjustments();
                self.update_window_buffer()?;
            }
            if self.window.is_key_down(Key::Left) {
                self.contrast = (self.contrast - 1).clamp(-255, 255);
                self.apply_adjustments();
                self.update_window_buffer()?;
            }

            // Toggle edge detection
            if self.window.is_key_pressed(Key::E, KeyRepeat::No) {
                self.edge_detection = !self.edge_detection;
                self.apply_adjustments();
                self.update_window_buffer()?;
            }

            // Reset adjustments
            if self.window.is_key_pressed(Key::R, KeyRepeat::No) {
                self.brightness = 0;
                self.contrast = 0;
                self.zoom = 1.0;
                self.edge_detection = false;
                self.buffer = self.original_buffer.clone();
                self.update_window_buffer()?;
            }

            // Show help
            if self.window.is_key_pressed(Key::H, KeyRepeat::No) {
                self.show_help();
            }

            // Show image info
            if self.window.is_key_pressed(Key::I, KeyRepeat::No) {
                self.show_info();
            }

            self.window.update();
        }

        Ok(())
    }
}

pub fn view_custom_image(path: &str) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let custom_image = CustomImage::from_bytes(&bytes)?;
    let mut viewer = ImageViewer::new(custom_image)?;
    viewer.run()?;
    Ok(())
}
