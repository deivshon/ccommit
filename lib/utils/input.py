from math import log10
from typing import Optional

import curses

__KEY_ENTER = 10
__KEY_CTRLW = 23
__ESC_KEY = 27
__KEY_BACKSPACE = 127

__RED_PAIR = 1
__YELLOW_PAIR = 2
__BLUE_PAIR = 3


def __remove_spaces_until_char(buf: str, screen: curses.window) -> str:
    while len(buf) > 0 and buf[-1] == " ":
        buf = buf[:-1]
        screen.addstr("\b \b")

    return buf


def __remove_chars_until_space(buf: str, screen: curses.window) -> str:
    while len(buf) > 0 and buf[-1] != " ":
        buf = buf[:-1]
        screen.addstr("\b \b")

    return buf


def __draw_current_length(buf: str, len_limit: int, screen: curses.window, prompt_length: int):
    for i in range(0, int(log10(len_limit)) + 1):
        screen.addstr(2, i, " ")

    char_count = str(len(buf))
    if len(buf) > 0.75 * len_limit:
        screen.addstr(2, 0, char_count,
                      curses.color_pair(__RED_PAIR) | curses.A_BOLD)
    elif len(buf) > 0.5 * len_limit:
        screen.addstr(2, 0, char_count,
                      curses.color_pair(__YELLOW_PAIR) | curses.A_BOLD)
    else:
        screen.addstr(2, 0, char_count)

    screen.move(1, len(buf) + prompt_length)


def input_detect_esc(
    prompt: Optional[str] = None,
    len_limit: Optional[int] = None,
    refuse_empty: bool = False,
    line_prompt: str = "> "
) -> Optional[str]:
    default_escdelay = curses.get_escdelay()
    stdscr = curses.initscr()

    curses.noecho()
    curses.set_escdelay(1)
    curses.start_color()
    curses.init_pair(__RED_PAIR, curses.COLOR_RED, curses.COLOR_BLACK)
    curses.init_pair(__YELLOW_PAIR, curses.COLOR_YELLOW, curses.COLOR_BLACK)
    curses.init_pair(__BLUE_PAIR, curses.COLOR_BLUE, curses.COLOR_BLACK)
    stdscr.keypad(True)

    if prompt is not None:
        stdscr.addstr(prompt + "\n")
    if len_limit is not None:
        stdscr.addstr(2, 0, "0")
        stdscr.move(1, 0)

    stdscr.addstr(line_prompt, curses.color_pair(__BLUE_PAIR) | curses.A_BOLD)

    buf = ""
    while True:
        key = stdscr.getch()

        if key == __ESC_KEY:
            buf = None
            break
        elif key == __KEY_ENTER:
            if refuse_empty and len(buf) == 0:
                continue

            break
        elif key == curses.KEY_BACKSPACE or key == __KEY_BACKSPACE:
            if len(buf) > 0:
                buf = buf[:-1]
                stdscr.addstr("\b \b")
        elif key == __KEY_CTRLW:
            if len(buf) == 0:
                continue
            elif buf[-1] == " ":
                buf = __remove_spaces_until_char(buf, stdscr)

            buf = __remove_chars_until_space(buf, stdscr)
        else:
            if len_limit is None or len(buf) < len_limit:
                buf += chr(key)
                stdscr.addstr(chr(key))

        if len_limit is not None:
            __draw_current_length(buf, len_limit, stdscr, len(line_prompt))

    stdscr.clear()
    curses.set_escdelay(default_escdelay)
    curses.endwin()

    return buf
