# Description
Attempt to make a simple clone of Google Drive.

The main goal was to try HTMX with Rust backend.

### Technologies used:
#### Frontend:
    - HTMX
    - Bootstrap CSS
    - JavaScript (event handling)
#### Backend:
    - Rust
    - Actix-Web
    - Handlebars (template rendering)

## Building:

### Raspberry Pi

#### Prerequisites
 - [`cross`](https://github.com/cross-rs/cross) installed
 - Docker daemon running

#### Steps
 1. Build custom docker image for `cross` tool to use by calling
 `docker buildx build --platform linux/armhf -t mydrive-raspberrypi-cross --load .`
 1. Run cross compilation 
 `RUSTFLAGS='-L /usr/arm-linux-gnueabihf/lib/ -L /usr/lib/arm-linux-gnueabihf/' cross build --release --target=armv7-unknown-linux-gnueabihf`
