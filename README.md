# pentas

pentas is a small browser built from scratch for educational purposes. Its core functionality is implemented without relying on external libraries, except for the GUI, which uses [gtk4](https://docs.gtk.org/gtk4/). While this is just a toy program and not intended for practical use, it loosely adheres to web standards.

![example_com](./demo/example_com.png)

## Install

### Linux (Debian)

```shell
sudo apt install libgtk-4-dev build-essential
```

### Mac (with homebrew)

```shell
brew install gtk4
```

For more detailed instructions, see [here](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation.html).

## Usage

```text
Usage: pentas [OPTIONS]

Options:
      --no-window-html <HTML>  The HTML file to parse in CLI mode
      --no-window-css <CSS>    The CSS file to parse in CLI mode
  -v, --verbose <LEVEL>        Set the verbosity level [default: quiet] [possible values: quiet, normal, verbose]
  -h, --help                   Print help
  -V, --version                Print version
```

### Run

To open the browser window:

```shell
cargo run
```

To open the browser window and visualize the tree structures built from HTML:

```shell
cargo run -- -v normal
```

To see how a CSS file is converted into a style sheet (No window):

```shell
cargo run -- --no-window-css <CSS file>
```
