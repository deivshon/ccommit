from math import log10
from typing import Optional

import curses


def input_detect_esc(
    prompt: Optional[str] = None,
    len_limit: Optional[int] = None,
    refuse_empty: bool = False
) -> Optional[str]:
    KEY_ENTER = 10
    KEY_CTRLW = 23
    ESC_KEY = 27
    KEY_BACKSPACE = 127

    RED_COLOR = 1
    YELLOW_COLOR = 2

    default_escdelay = curses.get_escdelay()
    stdscr = curses.initscr()

    curses.noecho()
    curses.set_escdelay(1)
    curses.start_color()
    curses.init_pair(RED_COLOR, curses.COLOR_RED, curses.COLOR_BLACK)
    curses.init_pair(YELLOW_COLOR, curses.COLOR_YELLOW, curses.COLOR_BLACK)
    stdscr.keypad(True)

    if prompt is not None:
        stdscr.addstr(prompt + "\n")
    if len_limit is not None:
        stdscr.addstr(2, 0, "0")
        stdscr.move(1, 0)

    buf = ""
    while True:
        key = stdscr.getch()

        if key == ESC_KEY:
            buf = None
            break
        elif key == KEY_ENTER:
            if refuse_empty and len(buf) == 0:
                continue

            break
        elif key == curses.KEY_BACKSPACE or key == KEY_BACKSPACE:
            if len(buf) > 0:
                buf = buf[:-1]
                stdscr.addstr("\b \b")
        elif key == KEY_CTRLW:
            if len(buf) == 0:
                continue
            elif buf[-1] == " ":
                while len(buf) > 0 and buf[-1] == " ":
                    buf = buf[:-1]
                    stdscr.addstr("\b \b")

            while len(buf) > 0 and buf[-1] != " ":
                buf = buf[:-1]
                stdscr.addstr("\b \b")
        else:
            if len_limit is None or len(buf) < len_limit:
                buf += chr(key)
                stdscr.addstr(chr(key))

        if len_limit is not None:
            for i in range(0, int(log10(len_limit)) + 1):
                stdscr.addstr(2, i, " ")

            char_count = str(len(buf))
            if len(buf) > 0.75 * len_limit:
                stdscr.addstr(2, 0, char_count, curses.color_pair(RED_COLOR))
            elif len(buf) > 0.5 * len_limit:
                stdscr.addstr(2, 0, char_count,
                              curses.color_pair(YELLOW_COLOR))
            else:
                stdscr.addstr(2, 0, char_count)

            stdscr.move(1, len(buf))

    stdscr.clear()
    curses.set_escdelay(default_escdelay)
    curses.endwin()

    return buf
