# `keylightd`

### Keyboard backlight daemon for Framework laptops
`keylightd` is a small system daemon for [Framework] laptops that listens to keyboard and touchpad input, and turns on the keyboard backlight while either is being used.

[Framework]: https://frame.work/

## Changes to original repo
- Fetch the current backlight brightness of the keyboard and restore to it instead of a preset
value
- touchpad inputs can now be ignored, so that the backlight only comes on when using the keyboard
- A twilight mode can be enabled that sets the "off" brightness to 1 instead of 0

## Requirements
For Intel based (11/12/13 gen) Framework laptops this should work with most
currently supported kernels. For AMD Ryzen based devices, Kernel 6.10 is needed
or you need to apply [a kernel patch](https://lore.kernel.org/chrome-platform/20231005160701.19987-1-dustin@howett.net/#t)
to whichever Kernel you are running.

## Installation

To install from source, clone the repository and run:

```shell
$ cargo build --release
$ sudo cp target/release/keylightd /usr/local/bin
```

`keylightd` has no native dependencies you have to install first (apart from a recent Rust toolchain for building it, of course).
It implements communication with the Embedded Controller itself, and talks to the input devices using `evdev` ioctls directly.
It also does not have any hard dependencies on a desktop environment or display server.

If you want to configure `keylightd` as a systemd service that starts on boot, you can use the provided service file:

```shell
$ sudo cp etc/keylightd.service /etc/systemd/system
$ sudo systemctl enable --now keylightd
```

## Running

Note that `keylightd` needs to be run as root, since it accesses the Embedded Controller to control the keyboard backlight.

`keylightd` takes the following command-line arguments:

```
Usage: keylightd [--brightness <brightness>] [--timeout <timeout>] [--power]

keylightd - automatic keyboard backlight daemon for Framework laptops

Options:
  --react-to-touchpad
                    also listen to touchpad events to enable/disable backlight 
                    [default=true]
  --timeout         activity timeout in seconds [default=10]
  --power           also control the power LED in the fingerprint module
  --twilight        reduce brightness to 1% instead of 0% on timeout
  --help            display usage information
```

If you're using the provided `keylightd.service` file, you can adjust the command line parameters there.

## Contributing

This project does not accept contributions. It is finished and does what I want of it.
