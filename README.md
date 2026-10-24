# esp32-sdcard
A library crate for the ESP32 and ESP-IDF to mount SD cards

## Hardware

There are two ways to access the SD card:

- `SdmmcCard` uses the 4 bit SDMMC interface which exists on ESP32/ESP32-S3 but **NOT** on ESP32-S2/ESP32-C3, e.g. on the ESP32-CAM.
  For detailed information on the wiring please consult the [official examples by espressif](https://github.com/espressif/esp-idf/tree/master/examples/storage/sd_card/sdmmc).
- `SdSpiCard` uses the SPI mode on a SPI bus exclusively used for the card, e.g. on the Inkplate boards.
  It is based on the SD SPI host driver of [esp-idf-hal](https://crates.io/crates/esp-idf-hal).

Both implement `SdCard` and can be mounted with `MountedFat::mount`.
However esp32-sdcard is only tested on an ESP32.

## Build

For the build process the pure cargo approach with [esp-idf-sys](https://crates.io/crates/esp-idf-sys) and [embuild](https://crates.io/crates/embuild) is used as described in [rust-esp32-std-demo](https://github.com/ivmarkov/rust-esp32-std-demo).

There is a tight dependency between the ESP-IDF version and the version of the Rust wrappers [esp-idf-sys](https://crates.io/crates/esp-idf-sys), [esp-idf-svc](https://crates.io/crates/esp-idf-svc) and [esp-idf-hal](https://crates.io/crates/esp-idf-hal).
To achieve reproducible builds the ESP-IDF version is fixed to a Tag inside [.cargo/config.toml](.cargo/config.toml) and not to a volatile release branch.

## Bright Sides

After mounting the SD card standard functions to open/read/modify files just work.

## Dark Sides

The first compatible partition is mounted.
Consecutive mount calls to the same already initialized SD card lead to the same partition mounted multiple times. (See also https://github.com/hammer90/esp32-file-server/commit/3ba3d1e3299c73564be4131628ef57835f81ab1c)


As suggested in the [official example](https://github.com/espressif/esp-idf/blob/master/examples/storage/sd_card/sdmmc/main/sd_card_example_main.c#L43-L46) the all in one setup-and-mount function `esp_vfs_fat_sdmmc_mount` is not used.
Instead the important parts of this function are rewritten in Rust.
This leads to some vague assumptions where to free memory as the C part sometimes takes over the cleanup and sometimes not.
Furthermore the companion `esp_vfs_fat_sdspi_mount` failed to initialize 3 of 3 tested cards in SPI mode on an ESP32-CAM.
So rewriting more parts in Rust would be beneficial.

## Example

See [esp32-file-server](https://github.com/hammer90/esp32-file-server/commit/3ba3d1e3299c73564be4131628ef57835f81ab1c) for an example of the SDMMC interface and [esp32-dashboard](https://github.com/hammer90/esp32-dashboard) for the SPI mode.

Last compiled with v1.90.0-nightly.
