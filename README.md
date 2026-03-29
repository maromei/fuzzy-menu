# Fuzzy Menu

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
- *3* The config file should consist of the following entries with the
  following meanings:

| name | meaning |
| ---- | ------- |
| name | title / human readable name |
| description | Long text description |
| tags | list of tags |

- *4* The TUI should be rendered using the `ratatui` rust library
- *4.1* The TUI should display a searchbox with the individual matches
  matches displayed below.
- *4.2* Each match should display the name in bold. Below the description.
  And below that a list of tags.

