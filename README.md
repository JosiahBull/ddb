# DDS (Disk Destroyer (is) Slow
DDB is a tool to restore data to sd-cards. It is optimized for writing to devices with high read speeds, and low write speeds, where the majority of the destination data will match the source data. This is useful for restoring an sd-card to a previous version. It works by having a read head move ahead of the write head in chunks, finding modified blocks and marking them to be overwritten. This will also reduce the wear on the sd-card.

This tool was written specifically for rolling back Jetson Nano sd-cards, but it should work for any device (e.g. Rapsberry Pi).

Note that for creating a disk image, it is still recommended to use `dd`, as it is more resource-efficient than this tool for creating a large block backup.

## Usage
```bash
# Create a backup of the sd-card
sudo dd if=/dev/sda of=$HOME/sda.img status=progress

# Restore the backup to the sd-card, in the event something goes wrong
sudo dds if=$HOME/sda.img of=/dev/sda
```

## Installation
```bash

```


## Compile from Source
```bash
# install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# install from github
cargo install dds --git https://github.com/JosiahBull/dds --force
```

## Contribution and Licensing

Contribution is welcomed, and will be licensed under the MIT license.