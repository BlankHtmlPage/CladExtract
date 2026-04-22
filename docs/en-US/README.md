# CladExtract

**CladExtract** is a fork of [RoExtract](https://github.com/AeEn123/RoExtract) - a tool for extracting cached assets from your Roblox installation.

## AI-Maintained Project

CladExtract is **100% maintained by an LLM** (currently MiniMax M2.5 and GLM 5.1). All code edits, PRs, and issues are processed by an AI assistant.

## About

CladExtract reads the Roblox client cache, SQLite database, and `rbx-storage` directory to list and extract cached assets including images, sounds, music, RBXM models, and KTX textures. The tool organizes extracted files into categorized folders and supports features like asset swapping, renaming, filtering, and bulk extraction.

## Building

Requires Rust 1.85+.

```bash
cargo build --release
```

## License

Same license as the original [RoExtract](https://github.com/AeEn123/RoExtract) project (MIT).
