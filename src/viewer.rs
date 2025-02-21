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

use minifb::{Window, WindowOptions, Key, Scale, KeyRepeat, MouseButton};
use crate::format::{CustomImage, ColorType};
use std::fs;
use std::error::Error;

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 10.0;
const ZOOM_STEP: f32 = 0.1;

/// A basic image viewer.
pub struct ImageViewer {
    window: Window,
    buffer: Vec<u32>,          // Processed (adjusted) image data
    original_buffer: Vec<u32>, // Original image data (RGB)
    width: usize,
    height: usize,
    zoom: f32,
    brightness: i32,
    contrast: i32,
    color_type: ColorType,
    pan_x: f32,                // Pan offset as fraction (0.0 to 1.0)
    pan_y: f32,                // Pan offset as fraction (0.0 to 1.0)
    edge_detection: bool,
}

impl ImageViewer {
    /// Create a new viewer using the provided custom image.
    /// The window size is set to the image dimensions.
    pub fn new(custom_image: CustomImage) -> Result<Self, Box<dyn Error>> {
        let width = custom_image.width as usize;
        let height = custom_image.height as usize;
        
        // Create the window with dimensions equal to the image.
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

        // Set FPS limit (~60 FPS)
        window.limit_update_rate(Some(std::time::Duration::from_micros(16_600)));

        // Convert the custom image's data into a u32 RGB buffer.
        let original_buffer = Self::convert_to_rgb(&custom_image);

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
            pan_x: 0.0,
            pan_y: 0.0,
            edge_detection: false,
        };

        // Apply initial adjustments and render.
        viewer.apply_adjustments();
        viewer.update_window_buffer()?;
        Ok(viewer)
    }

    /// Converts the custom image pixel data to a 32-bit RGB buffer.
    fn convert_to_rgb(image: &CustomImage) -> Vec<u32> {
        let mut buffer = vec![0u32; (image.width as usize) * (image.height as usize)];
        match image.color_type {
            ColorType::Gray => {
                for i in 0..buffer.len() {
                    let pixel = image.data[i] as u32;
                    buffer[i] = (pixel << 16) | (pixel << 8) | pixel;
                }
            }
            ColorType::Rgb => {
                for (i, chunk) in image.data.chunks_exact(3).enumerate() {
                    let r = chunk[0] as u32;
                    let g = chunk[1] as u32;
                    let b = chunk[2] as u32;
                    buffer[i] = (r << 16) | (g << 8) | b;
                }
            }
        }
        buffer
    }

    /// Applies brightness and contrast adjustments to the image.
    fn apply_adjustments(&mut self) {
        // Start from the original image.
        self.buffer = self.original_buffer.clone();
        if self.edge_detection {
            self.apply_edge_detection();
            return;
        }
        // Adjust each pixel.
        for pixel in self.buffer.iter_mut() {
            let r = (((*pixel >> 16) & 0xFF) as i32 + self.brightness).clamp(0, 255);
            let g = (((*pixel >> 8) & 0xFF) as i32 + self.brightness).clamp(0, 255);
            let b = (((*pixel) & 0xFF) as i32 + self.brightness).clamp(0, 255);
            let contrast = self.contrast.clamp(-255, 255);
            let factor = (259.0 * (contrast as f32 + 255.0)) / (255.0 * (259.0 - contrast as f32));
            let r_adj = (factor * (r as f32 - 128.0) + 128.0).clamp(0.0, 255.0) as u32;
            let g_adj = (factor * (g as f32 - 128.0) + 128.0).clamp(0.0, 255.0) as u32;
            let b_adj = (factor * (b as f32 - 128.0) + 128.0).clamp(0.0, 255.0) as u32;
            *pixel = (r_adj << 16) | (g_adj << 8) | b_adj;
        }
    }

    /// Applies a Sobel edge detection filter.
    fn apply_edge_detection(&mut self) {
        let mut grayscale = vec![0u8; self.width * self.height];
        for (i, &pixel) in self.original_buffer.iter().enumerate() {
            let r = ((pixel >> 16) & 0xFF) as u32;
            let g = ((pixel >> 8) & 0xFF) as u32;
            let b = (pixel & 0xFF) as u32;
            grayscale[i] = ((0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as i32 + self.brightness)
                          .clamp(0, 255) as u8;
        }
        let sobel_x = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
        let sobel_y = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];
        let mut edges = vec![0u32; self.width * self.height];
        for y in 1..(self.height - 1) {
            for x in 1..(self.width - 1) {
                let mut gx = 0;
                let mut gy = 0;
                for ky in 0..3 {
                    for kx in 0..3 {
                        let pixel = grayscale[(y + ky - 1) * self.width + (x + kx - 1)] as i32;
                        gx += pixel * sobel_x[ky][kx];
                        gy += pixel * sobel_y[ky][kx];
                    }
                }
                let magnitude = ((gx * gx + gy * gy) as f32).sqrt() as u32;
                let edge = if magnitude > 50 { 255 } else { 0 };
                edges[y * self.width + x] = (edge << 16) | (edge << 8) | edge;
            }
        }
        self.buffer = edges;
    }

    /// Performs bilinear interpolation on one channel.
    fn bilinear_interpolate(p00: u32, p10: u32, p01: u32, p11: u32, fx: f32, fy: f32) -> u32 {
        let interp0 = p00 as f32 * (1.0 - fx) + p10 as f32 * fx;
        let interp1 = p01 as f32 * (1.0 - fx) + p11 as f32 * fx;
        (interp0 * (1.0 - fy) + interp1 * fy).round() as u32
    }

    /// Updates the window buffer by scaling, panning, and interpolating.
    fn update_window_buffer(&mut self) -> Result<(), Box<dyn Error>> {
        let (win_width, win_height) = self.window.get_size();
        let scaled_width = (self.width as f32 * self.zoom) as usize;
        let scaled_height = (self.height as f32 * self.zoom) as usize;

        // Calculate maximum pan offsets.
        let max_pan_x = if scaled_width > win_width { scaled_width as i32 - win_width as i32 } else { 0 };
        let max_pan_y = if scaled_height > win_height { scaled_height as i32 - win_height as i32 } else { 0 };
        // Compute pixel offset from pan fractions.
        let offset_x = ((self.pan_x * scaled_width as f32) as i32).clamp(0, max_pan_x);
        let offset_y = ((self.pan_y * scaled_height as f32) as i32).clamp(0, max_pan_y);

        let mut new_buffer = vec![0u32; win_width * win_height];
        for win_y in 0..win_height {
            for win_x in 0..win_width {
                let img_x = (win_x as i32 + offset_x) as f32 / self.zoom;
                let img_y = (win_y as i32 + offset_y) as f32 / self.zoom;
                if img_x < 0.0 || img_y < 0.0 || img_x >= (self.width - 1) as f32 || img_y >= (self.height - 1) as f32 {
                    continue;
                }
                let x0 = img_x.floor() as usize;
                let y0 = img_y.floor() as usize;
                let x1 = (x0 + 1).min(self.width - 1);
                let y1 = (y0 + 1).min(self.height - 1);
                let fx = img_x - x0 as f32;
                let fy = img_y - y0 as f32;
                let p00 = self.buffer[y0 * self.width + x0];
                let p10 = self.buffer[y0 * self.width + x1];
                let p01 = self.buffer[y1 * self.width + x0];
                let p11 = self.buffer[y1 * self.width + x1];
                let r = Self::bilinear_interpolate((p00 >> 16) & 0xFF, (p10 >> 16) & 0xFF,
                                                   (p01 >> 16) & 0xFF, (p11 >> 16) & 0xFF, fx, fy);
                let g = Self::bilinear_interpolate((p00 >> 8) & 0xFF, (p10 >> 8) & 0xFF,
                                                   (p01 >> 8) & 0xFF, (p11 >> 8) & 0xFF, fx, fy);
                let b = Self::bilinear_interpolate(p00 & 0xFF, p10 & 0xFF,
                                                   p01 & 0xFF, p11 & 0xFF, fx, fy);
                new_buffer[win_y * win_width + win_x] = (r << 16) | (g << 8) | b;
            }
        }
        self.window.update_with_buffer(&new_buffer, win_width, win_height)
            .map_err(|e| format!("Window buffer update failed: {}", e))?;
        Ok(())
    }

    /// Main loop: handles input (keyboard and mouse) and updates the display.
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.show_help();
        let mut last_win_size = self.window.get_size();
        // Variables for mouse dragging.
        let mut last_mouse_pos: Option<(f32, f32)> = None;

        while self.window.is_open() && !self.window.is_key_down(Key::Escape) {
            let mut needs_update = false;

            // Process keyboard input.
            for key in self.window.get_keys_pressed(KeyRepeat::Yes) {
                match key {
                    Key::H => self.show_help(),
                    Key::I => self.show_info(),
                    Key::E => { self.edge_detection = !self.edge_detection; needs_update = true; }
                    Key::R => {
                        self.brightness = 0;
                        self.contrast = 0;
                        self.zoom = 1.0;
                        self.pan_x = 0.0;
                        self.pan_y = 0.0;
                        self.edge_detection = false;
                        needs_update = true;
                    }
                    Key::Equal | Key::NumPadPlus => { self.zoom = (self.zoom + ZOOM_STEP).min(MAX_ZOOM); needs_update = true; }
                    Key::Minus | Key::NumPadMinus => { self.zoom = (self.zoom - ZOOM_STEP).max(MIN_ZOOM); needs_update = true; }
                    Key::Up => { self.brightness = (self.brightness + 5).min(255); needs_update = true; }
                    Key::Down => { self.brightness = (self.brightness - 5).max(-255); needs_update = true; }
                    Key::Right => { self.contrast = (self.contrast + 5).min(255); needs_update = true; }
                    Key::Left => { self.contrast = (self.contrast - 5).max(-255); needs_update = true; }
                    _ => {}
                }
            }

            // Handle mouse dragging for panning.
            if self.window.get_mouse_down(MouseButton::Left) {
                if let Some((cur_x, cur_y)) = self.window.get_mouse_pos(minifb::MouseMode::Discard) {
                    if let Some((last_x, last_y)) = last_mouse_pos {
                        let dx = cur_x - last_x;
                        let dy = cur_y - last_y;
                        // Update pan offsets as fractions.
                        self.pan_x = (self.pan_x + dx / (self.width as f32 * self.zoom)).clamp(0.0, 1.0);
                        self.pan_y = (self.pan_y + dy / (self.height as f32 * self.zoom)).clamp(0.0, 1.0);
                        needs_update = true;
                    }
                    last_mouse_pos = Some((cur_x, cur_y));
                }
            } else {
                last_mouse_pos = None;
            }

            // Check for window resize.
            let current_size = self.window.get_size();
            if current_size != last_win_size {
                last_win_size = current_size;
                needs_update = true;
            }

            if needs_update {
                self.apply_adjustments();
                self.update_window_buffer()?;
            }
            self.window.update();
            std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
        }
        Ok(())
    }

    /// Displays help information.
    fn show_help(&self) {
        println!("\nImage Viewer Controls:");
        println!("----------------------");
        println!("ESC - Exit");
        println!("H   - Show help");
        println!("+ / - - Zoom in/out (use mouse scroll if supported)");
        println!("↑ / ↓ - Adjust brightness");
        println!("← / → - Adjust contrast");
        println!("E   - Toggle edge detection");
        println!("R   - Reset adjustments");
        println!("I   - Show image info");
        println!("Drag with left mouse button to pan");
    }

    /// Displays image information.
    fn show_info(&self) {
        println!("\nImage Information:");
        println!("------------------");
        println!("Dimensions: {}x{}", self.width, self.height);
        println!("Color Type: {:?}", self.color_type);
        println!("Zoom: {:.1}x", self.zoom);
        println!("Brightness: {}", self.brightness);
        println!("Contrast: {}", self.contrast);
        println!("Edge Detection: {}", if self.edge_detection { "On" } else { "Off" });
        let (win_w, win_h) = self.window.get_size();
        println!("Window size: {}x{}", win_w, win_h);
    }
}

/// Entry point: loads a custom image file and starts the viewer.
pub fn view_custom_image(path: &str) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let custom_img = CustomImage::from_bytes(&bytes)?;
    let mut viewer = ImageViewer::new(custom_img)?;
    viewer.run()
}
