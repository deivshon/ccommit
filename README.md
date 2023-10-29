# ccommit

Conventional commits from the terminal

## Arguments

| Argument | Long argument | Description                            | Optional | Default                       |
| -------- | ------------- | -------------------------------------- | -------- | ----------------------------- |
| -C       | --working-dir | Set the working directory              | Yes      | .                             |
| -c       | --config      | Path to the desired configuration file | Yes      | ~/.config/ccommit/config.json |

## Limitations

-   Can't enter more than one line on text input requests
-   No actual emojis in the gitmoji selection menu because of related [simple-term-menu issue](https://github.com/IngoMeyer441/simple-term-menu/issues/73)
