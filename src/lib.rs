#![allow(clippy::single_component_path_imports)]

use std::ffi::{c_void, CString};

use anyhow::{bail, Result};
use log::*;

use esp_idf_hal::gpio::{Gpio12, Gpio13, Gpio14, Gpio15, Gpio2, Gpio4, InputOutput, PinDriver};
use esp_idf_sys::{
    self, esp_vfs_fat_register, esp_vfs_fat_unregister_path, f_mount, ff_diskio_get_drive,
    ff_diskio_register, ff_diskio_register_sdmmc, free, malloc, memcpy, sdmmc_card_init,
    sdmmc_card_t, sdmmc_host_deinit, sdmmc_host_do_transaction, sdmmc_host_get_dma_info,
    sdmmc_host_get_real_freq, sdmmc_host_get_slot_width, sdmmc_host_init, sdmmc_host_init_slot,
    sdmmc_host_io_int_enable, sdmmc_host_io_int_wait, sdmmc_host_set_bus_ddr_mode,
    sdmmc_host_set_bus_width, sdmmc_host_set_card_clk, sdmmc_host_set_cclk_always_on,
    sdmmc_host_set_input_delay, sdmmc_host_t, sdmmc_slot_config_t,
    sdmmc_slot_config_t__bindgen_ty_1, sdmmc_slot_config_t__bindgen_ty_2, FATFS,
};

pub struct SdPins {
    pub cmd: Gpio15,
    pub clk: Gpio14,
    pub d0: Gpio2,
    pub d1: Gpio4,
    pub d2: Gpio12,
    pub d3: Gpio13,
}

struct PinDrivers<'a> {
    cmd: PinDriver<'a, Gpio15, InputOutput>,
    clk: PinDriver<'a, Gpio14, InputOutput>,
    d0: PinDriver<'a, Gpio2, InputOutput>,
    d1: PinDriver<'a, Gpio4, InputOutput>,
    d2: PinDriver<'a, Gpio12, InputOutput>,
    d3: PinDriver<'a, Gpio13, InputOutput>,
}

pub struct SdmmcCard<'a> {
    card: *mut sdmmc_card_t,
    _host_config: sdmmc_host_t,
    _pins: PinDrivers<'a>,
}

impl<'a> SdmmcCard<'a> {
    pub fn new(pins: SdPins) -> Result<Self> {
        let pins = PinDrivers {
            cmd: PinDriver::input_output(pins.cmd)?,
            clk: PinDriver::input_output(pins.clk)?,
            d0: PinDriver::input_output(pins.d0)?,
            d1: PinDriver::input_output(pins.d1)?,
            d2: PinDriver::input_output(pins.d2)?,
            d3: PinDriver::input_output(pins.d3)?,
        };
        unsafe {
            let err = sdmmc_host_init();
            if err != 0 {
                bail!("failed to init sdmmc_host {}", err);
            }
            let host_config = sdmmc_host_t {
                flags: 23,
                slot: 1,
                max_freq_khz: 20000,
                io_voltage: 3.3,
                init: Some(sdmmc_host_init),
                set_bus_width: Some(sdmmc_host_set_bus_width),
                get_bus_width: Some(sdmmc_host_get_slot_width),
                set_bus_ddr_mode: Some(sdmmc_host_set_bus_ddr_mode),
                set_card_clk: Some(sdmmc_host_set_card_clk),
                set_cclk_always_on: Some(sdmmc_host_set_cclk_always_on),
                do_transaction: Some(sdmmc_host_do_transaction),
                __bindgen_anon_1: esp_idf_sys::sdmmc_host_t__bindgen_ty_1 {
                    deinit: Some(sdmmc_host_deinit),
                },
                io_int_enable: Some(sdmmc_host_io_int_enable),
                io_int_wait: Some(sdmmc_host_io_int_wait),
                command_timeout_ms: 0,
                get_real_freq: Some(sdmmc_host_get_real_freq),
                input_delay_phase: 0,
                set_input_delay: Some(sdmmc_host_set_input_delay),
                dma_aligned_buffer: std::ptr::null_mut(),
                pwr_ctrl_handle: std::ptr::null_mut(),
                get_dma_info: Some(sdmmc_host_get_dma_info),
            };
            let slot_config = sdmmc_slot_config_t {
                __bindgen_anon_1: sdmmc_slot_config_t__bindgen_ty_1 { gpio_cd: -1 },
                __bindgen_anon_2: sdmmc_slot_config_t__bindgen_ty_2 { gpio_wp: -1 },
                width: 4,
                flags: 0,
                clk: pins.clk.pin(),
                cmd: pins.cmd.pin(),
                d0: pins.d0.pin(),
                d1: pins.d1.pin(),
                d2: pins.d2.pin(),
                d3: pins.d3.pin(),
                d4: -1,
                d5: -1,
                d6: -1,
                d7: -1,
            };
            let pslot_config: *const sdmmc_slot_config_t = &slot_config;
            // configures pins (again)
            let err = sdmmc_host_init_slot(host_config.slot, pslot_config);
            if err != 0 {
                sdmmc_host_deinit();
                bail!("failed to sdmmc_host_init_slot {}", err);
            }
            let size = std::mem::size_of::<sdmmc_card_t>();
            let card = malloc(size.try_into().unwrap_or(136)) as *mut sdmmc_card_t;
            if card.is_null() {
                sdmmc_host_deinit();
                bail!("failed to allocate memory");
            }
            let phost_config: *const sdmmc_host_t = &host_config;
            // clears memory of pcard, copies host_config and initializes the card
            let err = sdmmc_card_init(phost_config, card);
            if err != 0 {
                sdmmc_host_deinit();
                free(card as *mut c_void);
                bail!("failed to sdmmc_card_init {}", err);
            }

            Ok(Self {
                card,
                _host_config: host_config,
                _pins: pins,
            })
        }
    }

    pub fn size(&self) -> i64 {
        unsafe {
            let capacity: i64 = (*self.card).csd.capacity.into();
            let sector_size: i64 = (*self.card).csd.sector_size.into();
            capacity * sector_size
        }
    }

    pub fn read_block_len(&self) -> i32 {
        unsafe { (*self.card).csd.read_block_len }
    }
}

impl<'a> Drop for SdmmcCard<'a> {
    fn drop(&mut self) {
        unsafe {
            sdmmc_host_deinit();
            free(self.card as *mut c_void);
        }
    }
}

pub struct MountedFat<'a> {
    _sdmmc_card: SdmmcCard<'a>,
    card: *mut sdmmc_card_t,
    base_path: CString,
    drv: u8,
    fat_drive: [u8; 3],
    fatfs: *mut FATFS,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FatFsStatistics {
    sectors_per_cluster: u16,
    sectors_per_fat: u32,
    sector_size: u16,
}

impl<'a> MountedFat<'a> {
    pub fn mount(sdmmc_card: SdmmcCard<'a>, mount_point: &str) -> Result<Self> {
        unsafe {
            let card_size: u32 = std::mem::size_of::<sdmmc_card_t>()
                .try_into()
                .unwrap_or(136);
            let card = malloc(card_size) as *mut sdmmc_card_t;
            if card.is_null() {
                bail!("failed to allocate memory");
            }

            memcpy(
                card as *mut c_void,
                sdmmc_card.card as *mut c_void,
                card_size,
            );

            let mut drv = 0xFF;
            let pdrv: *mut u8 = &mut drv;
            // get next free drive slot
            let err = ff_diskio_get_drive(pdrv);
            if err != 0 || drv == 0xFF {
                free(card as *mut c_void);
                bail!("failed to ff_diskio_get_drive {} {}", err, drv);
            }
            // registers sdmmc driver for this disk, copies pcard (pointer only, not mem) to internal storage
            ff_diskio_register_sdmmc(drv, card);

            let mut pfatfs: *mut FATFS = std::ptr::null_mut();
            let ppfatfs: *mut *mut FATFS = &mut pfatfs;
            let fat_drive: [u8; 3] = [(0x30 + drv).try_into().unwrap(), 0x3a, 0];
            let base_path = CString::new(mount_point)?;
            // connect base_path to fat_drive and allocate memory for fatfs
            let err = esp_vfs_fat_register(base_path.as_ptr(), &fat_drive as *const u8, 8, ppfatfs);
            if err != 0 {
                ff_diskio_register(drv, std::ptr::null());
                free(card as *mut c_void);
                bail!("failed to esp_vfs_fat_register {}", err);
            }
            // finally mount first FAT32 partition
            let err = f_mount(pfatfs, base_path.as_ptr(), 1);
            if err != 0 {
                ff_diskio_register(drv, std::ptr::null());
                let err = esp_vfs_fat_unregister_path(base_path.as_ptr());
                if err != 0 {
                    warn!("failed to esp_vfs_fat_unregister_path {}", err);
                }
                free(card as *mut c_void);
                bail!("failed to f_mount {}", err);
            }

            Ok(Self {
                _sdmmc_card: sdmmc_card,
                card,
                base_path,
                drv,
                fat_drive,
                fatfs: pfatfs,
            })
        }
    }

    pub fn statistics(&self) -> FatFsStatistics {
        unsafe {
            FatFsStatistics {
                sectors_per_cluster: (*self.fatfs).csize,
                sector_size: (*self.fatfs).ssize,
                sectors_per_fat: (*self.fatfs).fsize,
            }
        }
    }
}

impl<'a> Drop for MountedFat<'a> {
    fn drop(&mut self) {
        unsafe {
            let err = f_mount(std::ptr::null_mut(), &self.fat_drive as *const u8, 0);
            if err != 0 {
                warn!("failed to unmount {}", err);
            }
            ff_diskio_register(self.drv, std::ptr::null());
            let err = esp_vfs_fat_unregister_path(self.base_path.as_ptr());
            if err != 0 {
                warn!("failed to esp_vfs_fat_unregister_path {}", err);
            }
            free(self.card as *mut c_void);
        }
    }
}
