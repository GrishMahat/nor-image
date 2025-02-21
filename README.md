This project was made just for fun. It has made me laugh, cry. I know the code is weird—please don’t use it for anything serious!

I’m learning Rust, and I wanted to understand how images work, so I built this. 

# Nor-Image

A high-performance image processing and conversion tool written in Rust. This tool provides efficient image format conversion with support for custom formats, compression, and real-time viewing capabilities.

### 🔄 Format Conversion
- **PNG ↔️ Custom Format (.nor)**
  - Lossless conversion between PNG and .nor format
  - Support for both RGB and grayscale color spaces
  - Metadata preservation

### 🗜️ Compression Options
- **Multiple Compression Methods**
  - Run-Length Encoding (RLE) - Best for images with large uniform areas
  - Delta Encoding - Efficient for gradual color changes
  - Lossy Compression - Configurable quality settings
  - No Compression - For maximum quality

### 🎨 Image Processing
- **Real-time Adjustments**
  - Brightness control (-255 to 255)
  - Contrast enhancement (-255 to 255)
  - High-quality image resizing
  - Edge detection with configurable sensitivity

### 🚀 Performance Features
- **Optimized Processing**
  - Parallel processing using Rayon
  - Efficient memory usage with streaming
  - Smart caching system
  - Configurable chunk-based processing

### 🖥️ Interactive Viewer
- **Real-time Controls**
  - Zoom: Mouse wheel or +/- keys
  - Pan: Arrow keys or mouse drag
  - Brightness/Contrast: Up/Down/Left/Right
  - Reset: R key
  - Edge Detection Toggle: E key
  - Help: H key

## 🚀 Quick Start

### Prerequisites
- Rust 1.56.0 or higher
- Cargo package manager

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/nor-image.git

# Navigate to project directory
cd nor-image

# Build the project (release mode for best performance)
cargo build --release

# Optional: Add to your PATH
cp target/release/nor-image ~/.local/bin/
```

### Basic Usage Examples
If you haven't added nor-image to your PATH, you can run commands using cargo:
```bash
cargo run -- 
```


1. **Convert PNG to NOR format:**
   ```bash
   nor-image png-to-custom input.png output.nor
   ```

2. **Convert with compression:**
   ```bash
   nor-image png-to-custom input.png output.nor --compression rle
   ```

3. **Convert with image processing:**
   ```bash
   nor-image png-to-custom input.png output.nor \
     --brightness 20 \
     --contrast 10 \
     --grayscale
   ```

4. **View an image:**
   ```bash
   nor-image view image.nor
   ```

### Advanced Usage

#### Compression Options
```bash
# RLE compression (best for logos, screenshots)
nor-image png-to-custom input.png output.nor --compression rle

# Delta compression (best for photographs)
nor-image png-to-custom input.png output.nor --compression delta

# Lossy compression with quality control
nor-image png-to-custom input.png output.nor --compression lossy
```

#### Image Processing
```bash
# Resize image
nor-image png-to-custom input.png output.nor --width 800 --height 600

# Convert to grayscale with adjustments
nor-image png-to-custom input.png output.nor \
  --grayscale \
  --brightness 30 \
  --contrast 20

# Optimize performance
nor-image png-to-custom input.png output.nor \
  --chunk-size 2 \
  --no-cache \
  --no-streaming
```

## 📦 Custom Format (.nor) Specification

The .nor format is designed for efficient storage and processing:

```
[Header]
- Magic Number (4 bytes): "CIMG"
- Version (1 byte)
- Color Type (1 byte): 0=Gray, 1=RGB
- Width (4 bytes, little-endian)
- Height (4 bytes, little-endian)
- Compression Type (1 byte)

[Metadata]
- Length (4 bytes)
- JSON data (variable length)

[Image Data]
- Compressed/Raw pixel data

[Footer]
- SHA256 checksum (32 bytes)
```

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines
- Follow Rust best practices and idioms
- Update documentation as needed
- Use descriptive commit messages

## 📝 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [image-rs](https://github.com/image-rs/image) - Rust image processing foundation
- [minifb](https://github.com/emoon/rust_minifb) - Minimal frame buffer window library
- [rayon](https://github.com/rayon-rs/rayon) - Data parallelism library
- [Image Processing Basics](https://en.wikipedia.org/wiki/Digital_image_processing)
- [Compression Algorithms](https://en.wikipedia.org/wiki/Image_compression)
