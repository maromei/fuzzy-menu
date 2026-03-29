# Fuzzy Menu

> [!IMPORTANT]
> **Disclaimer:** This is a fully vibe coded application which was created
> on a whim.
> There is no real quality or much thought expected here.

This project implements a very small TUI for fuzzy searching and executing
commands defined in `config.toml`. See the `test_config.toml` for
an example.

The SPEC section below defines how the tool is supposed to work.

## Installation

```bash
cargo install --git https://github.com/maromei/fuzzy-menu/
```

## SPEC

Please build me a rust project called `fuzzy-menu`.
The goal of the tool is to create a menu in the tui with which we can fuzzy
find commands defined in a config.toml file.

The following features / specs need to be implemented:

- *1* The tool should use the `fzf` tool in the background.
- *1.1* Every entry in the config file (see point *2* and *3*) should be
  fuzzy searchable.
- *2* Items should be read from a `config.toml` file.
- *2.1* The config file lives in the `$HOME/.config/fuzzy-menu/` directory.
- *2.2* There exists a CLI argument with which the config file can be loaded
  from the path passed as an argument.
- *2.3* If the `$HOME/.config/fuzzy-menu/` directory and the `config.toml` file
  does not exist on program start, create it.
- *3* The config file should consist have different commands under different
  headings / entry-keys.
- *3.1* The entry-keys are machine readable.
- *3.2* The general format should look like this:

```toml
[entry-key]
name = "Some Human readable name"
description = "Some long description about what all this does."
tags = [
    "some",
    "demo",
    "entry"
]

[some-other-entry-key]
description = "This one does not have human readable name."
tags = [
    "some",
    "demo",
    "entry"
]
```

- *3.3* The following fields can be specified for the entries with the
  following meanings:


| name | meaning |
| ---- | ------- |
| command | The command to be executed |
| name | optional: title / human readable name. If not present use the entry-key |
| description | optional: Long text description |
| tags | optional: list of tags |

- *3.4* The description field should allow for multiline input, which is
  also displayed as multiline input in the terminal.
- *3.4.1* Should the multiline start with a linebreak followed by some
  amount of whitespace, the amount of of whitespace will be removed at the
  start of each following line. F.e.:


```toml
...
description = """
    This is some multiline string.
    The indentation will be removed on each line.
        Since this line has an additional indentation level, this additional
        one will be displayed.
"""
...
```

Should render to:

```text
This is some multiline string.
The indentation will be removed on each line.
    Since this line has an additional indentation level, this additional
    one will be displayed.
```

- *4* The TUI should be rendered using the `ratatui` rust library
- *4.1* The TUI should display a searchbox with the individual matches
  matches displayed below.
- *4.2* Each match should display the name in bold. Below the description.
  And below that a list of tags.
- *4.3* Should no human readable name be defined, use the `entry-key` to
  be displayed in bold instead.
- *5* The following keymaps should apply:

| key | mode | action |
| --- | ---- | ------ |
| `ESC` | insert | switch to normal mode |
| `CTRL-c` | insert | exit |
| `CTRL-n` | insert | select the next item in the `match-box` |
| `CTRL-y` | insert | select the previouse item in the `match-box` |
| `DOWN_ARROW` | insert | select the next item in the `match-box` |
| `UP_ARROW` | insert | select the previouse item in the `match-box` |
| `ENTER` | insert | execute the currently selected entry |
| `i` | normal | switch to insert mode |
| `j` | normal | select the next item in the `match-box` |
| `k` | normal | select the previouse item in the `match-box` |
| `ESC` | normal | exit |
| `CTRL-c` | normal | exit |
| `ENTER` | normal | execute the currently selected entry |

- *5.1* The tool has two modes that are similar to nvim: `insert` and `normal`
- *5.2* When first starting the tool, the `insert` mode is active.
- *5.3* In `insert` mode, text should be able to be typed normally.
- *5.4* In `normal` mode, text does not get passed to the searchbox, and other
  keymaps specified above apply.
- *6* Executing an entry should execute the `command` in the current shell
  environment and session.

