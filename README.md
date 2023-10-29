# ccommit

Conventional commits from the terminal

## Arguments

| Argument | Long argument | Description                            | Optional | Default                       |
| -------- | ------------- | -------------------------------------- | -------- | ----------------------------- |
| -C       | --working-dir | Set the working directory              | Yes      | .                             |
| -c       | --config      | Path to the desired configuration file | Yes      | ~/.config/ccommit/config.json |

## Configuration file

The default configuration file path is `~/.config/ccommit/config.json`

Below is a table with the configuration options

| Key        | Type    | Description                                | Default |
| ---------- | ------- | ------------------------------------------ | ------- |
| use_emojis | boolean | Use actual emojis instead of gitmoji codes | false   |

## Limitations

-   Can't enter more than one line on text input requests
-   No actual emojis in the gitmoji selection menu because of related [simple-term-menu issue](https://github.com/IngoMeyer441/simple-term-menu/issues/73)
