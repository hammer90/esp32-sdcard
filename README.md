# esp32-sdcard
A library crate for the ESP32 and ESP-IDF to mount SD cards

## Hardware

For access to the SD card the SDMMC interface is used which exists on ESP32/ESP32-S3 but **NOT** on ESP32-S2/ESP32-C3.
However esp32-sdcard is only tested on an ESP32.
For detailed information on the wiring please consult the [official examples by espressif](https://github.com/espressif/esp-idf/tree/master/examples/storage/sd_card/sdmmc).

## Build

For the build process the pure cargo approach with [esp-idf-sys](https://crates.io/crates/esp-idf-sys) and [embuild](https://crates.io/crates/embuild) is used as described in [rust-esp32-std-demo](https://github.com/ivmarkov/rust-esp32-std-demo).

There is a tight dependency between the ESP-IDF version and the version of the Rust wrappers [esp-idf-sys](https://crates.io/crates/esp-idf-sys), [esp-idf-svc](https://crates.io/crates/esp-idf-svc) and [esp-idf-hal](https://crates.io/crates/esp-idf-hal).
To achieve reproducible builds the ESP-IDF version is fixed to a Tag inside [.cargo/config.toml](.cargo/config.toml) and not to a volatile release branch.

## Bright Sides

After mounting the SD card standard functions to open/read/modify files just work.

## Dark Sides

The first compatible partition is mounted.
Consecutive mount calls to the same already initialized SD card lead to the same partition mounted multiple times.


As suggested in the [official example](https://github.com/espressif/esp-idf/blob/master/examples/storage/sd_card/sdmmc/main/sd_card_example_main.c#L43-L46) the all in one setup-and-mount function `esp_vfs_fat_sdmmc_mount` is not used.
Instead the important parts of this function are rewritten in Rust.
This leads to some vague assumptions where to free memory as the C part sometimes takes over the cleanup and sometimes not.
Furthermore the companion `esp_vfs_fat_sdspi_mount` fails to initialize 3 of 3 tested cards in SPI mode.
So rewriting more parts in Rust would be beneficial.
