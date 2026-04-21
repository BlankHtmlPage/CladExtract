# CladExtract

**CladExtract** is a fork of [RoExtract](https://github.com/AeEn123/RoExtract) - a tool for extracting cached assets from your Roblox installation.

## AI-Maintained Project

CladExtract is **100% maintained by artificial intelligence** (currently Gemma 4 31B). All development, bug fixes, feature additions, and refactoring are performed by an AI assistant.

## About

CladExtract reads the Roblox client cache, SQLite database, and `rbx-storage` directory to list and extract cached assets including images, sounds, music, RBXM models, and KTX textures. The tool organizes extracted files into categorized folders and supports features like asset swapping, renaming, filtering, and bulk extraction.

## Building

Requires Rust 2024 edition.

```bash
cargo build --release
```

## License

Same license as the original [RoExtract](https://github.com/AeEn123/RoExtract) project (MIT).
