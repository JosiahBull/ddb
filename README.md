# DDS Disk Destroyer (is) Slow

![Test](https://github.com/josiahbull/dds/actions/workflows/precommit.yaml/badge.svg)
![Build](https://github.com/josiahbull/dds/actions/workflows/prerelease.yml/badge.svg)

DDS is a tool to restore data to sd-cards. It is optimized for writing to
devices with high read speeds, and low write speeds, where the majority of
the destination data will match the source data. This is useful for
restoring an sd-card to a previous version. It works by having a read head
move ahead of the write head in chunks, finding modified blocks and marking
them to be overwritten. This will also reduce the wear on the sd-card.

This tool was written specifically for rolling back Jetson Nano sd-cards, but
it should work for any device (e.g. Rapsberry Pi).

Note that for creating a disk image, it is still recommended to use `dd`, as
it is more resource-efficient than this tool for creating a large block backup.

This tool does support multithreading, using separate processes for reading and
writing. This isn't especially useful in 99% of situations - but if you're
expecting >70% of your sd card to be overwritten it could be useful to enable.

## Usage

```bash
# Create a backup of the sd-card
sudo dd if=/dev/sda of=$HOME/sda.img status=progress

# Restore the backup to the sd-card
sudo dds --input=$HOME/sda.img --output=/dev/sda
```

## Installation

```bash
curl https://raw.githubusercontent.com/josiahbull/dds/main/install.sh | bash
```

## Compile from Source

```bash
# install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# clone and build from source
git clone https://github.com/josiahbull/dds/
cd dds
cargo build --release
./target/release/dds --help
```

## Contribution and Licensing

Contribution is welcomed, and will be licensed under the MIT license.
