from math import log10
from typing import Optional, Tuple, Union
from enum import Enum
import curses


class Direction(Enum):
    RIGHT = 1
    LEFT = -1


class UnknownEscapeSequence(Enum):
    UNKNOWN = 0


__KEY_ENTER = 10
__KEY_CTRLW = 23
__ESC_KEY = 27
__KEY_BACKSPACE = 127
__KEY_RIGHT = 261
__KEY_LEFT = 260
__KEY_CTRL_LEFT = 552
__KEY_CTRL_RIGHT = 567
__KEY_CTRL_A = 1
__KEY_CTRL_E = 5

__RED_PAIR = 1
__YELLOW_PAIR = 2
__BLUE_PAIR = 3
__GREEN_PAIR = 4

__INPUT_Y = 1

__SEQ_ONLY_ESC = (-1, -1, -1, -1, -1)
__SEQ_CTRL_RIGHT = (91, 49, 59, 53, 67)
__SEQ_CTRL_LEFT = (91, 49, 59, 53, 68)


def __is_ascii(key: int) -> bool:
    return 32 <= key <= 126


def __remove_single_char(buf: str, pos: int, screen: curses.window) -> Tuple[str, int]:
    buf = buf[0:max(pos - 1, 0)] + buf[pos:len(buf)]
    screen.addstr("\b \b")
    pos -= 1
    return buf, pos


def __remove_spaces_until_char(buf: str, pos: int, screen: curses.window) -> Tuple[str, int]:
    while len(buf) > (pos - 1) and pos > 0 and buf[pos - 1] == " ":
        buf, pos = __remove_single_char(buf, pos, screen)

    return buf, pos


def __remove_chars_until_space(buf: str, pos: int, screen: curses.window) -> Tuple[str, int]:
    while len(buf) > (pos - 1) and pos > 0 and buf[pos - 1] != " ":
        buf, pos = __remove_single_char(buf, pos, screen)

    return buf, pos


def __skip_chars_until_space(buf: str, pos: int, direction: Direction) -> int:
    if direction == Direction.LEFT:
        while len(buf) > (pos - 1) and pos > 0 and buf[pos - 1] != " ":
            pos -= 1
    else:
        while pos < len(buf) and buf[pos] != " ":
            pos += 1

    return pos


def __skip_chars_until_char(buf: str, pos: int, direction: Direction) -> int:
    if direction == Direction.LEFT:
        while len(buf) > (pos - 1) and pos > 0 and buf[pos - 1] == " ":
            pos -= 1
    else:
        while pos < len(buf) and buf[pos] == " ":
            pos += 1

    return pos


def __handle_ctrl_left(buf: str, pos: int) -> int:
    if pos == 0:
        return pos

    if buf[pos - 1] == " ":
        pos = __skip_chars_until_char(buf, pos, Direction.LEFT)

    pos = __skip_chars_until_space(buf, pos, Direction.LEFT)

    return pos


def __handle_ctrl_right(buf: str, pos: int) -> int:
    if pos >= len(buf):
        return pos

    if buf[pos] == " ":
        pos = __skip_chars_until_char(buf, pos, Direction.RIGHT)

    pos = __skip_chars_until_space(buf, pos, Direction.RIGHT)

    return pos


def __handle_escape_seq(buf: str, pos: int, screen: curses.window) -> Optional[int] | UnknownEscapeSequence:
    screen.nodelay(True)
    sequence = (
        screen.getch(),
        screen.getch(),
        screen.getch(),
        screen.getch(),
        screen.getch()
    )
    screen.nodelay(False)

    if sequence == __SEQ_CTRL_LEFT:
        pos = __handle_ctrl_left(buf, pos)
    elif sequence == __SEQ_CTRL_RIGHT:
        pos = __handle_ctrl_right(buf, pos)
    elif sequence == __SEQ_ONLY_ESC:
        return None
    else:
        return UnknownEscapeSequence.UNKNOWN

    return pos


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
        screen.addstr(2, 0, char_count,
                      curses.color_pair(__GREEN_PAIR) | curses.A_BOLD)

    screen.move(1, len(buf) + prompt_length)


def __draw_buf(
    buf: str,
    pos: int,
    screen: curses.window,
    line_limit: int,
    prompt_length: int
):
    screen.move(__INPUT_Y, prompt_length)
    screen.addstr(" " * line_limit)
    screen.move(__INPUT_Y, prompt_length)
    screen.addstr(buf)
    screen.move(__INPUT_Y, prompt_length + pos)


def input_detect_esc(
    prompt: Optional[str] = None,
    len_limit: Optional[int] = None,
    refuse_empty: bool = False,
    line_prompt: str = "> "
) -> Optional[str]:
    default_escdelay = curses.get_escdelay()
    stdscr = curses.initscr()

    _, max_x = stdscr.getmaxyx()
    if len_limit is not None:
        if len_limit > max_x:
            max_x -= 1
            len_limit = max_x
        else:
            max_x = len_limit
    else:
        max_x -= 1

    curses.noecho()
    curses.set_escdelay(1)
    curses.start_color()
    curses.init_pair(__RED_PAIR, curses.COLOR_RED, curses.COLOR_BLACK)
    curses.init_pair(__YELLOW_PAIR, curses.COLOR_YELLOW, curses.COLOR_BLACK)
    curses.init_pair(__BLUE_PAIR, curses.COLOR_BLUE, curses.COLOR_BLACK)
    curses.init_pair(__GREEN_PAIR, curses.COLOR_GREEN, curses.COLOR_BLACK)
    stdscr.keypad(True)

    if prompt is not None:
        stdscr.addstr(prompt + "\n")
    if len_limit is not None:
        stdscr.addstr(2, 0, "0", curses.color_pair(
            __GREEN_PAIR) | curses.A_BOLD)
        stdscr.move(1, 0)

    stdscr.addstr(line_prompt, curses.color_pair(__BLUE_PAIR) | curses.A_BOLD)

    buf = ""
    pos = 0
    while True:
        key = stdscr.getch()

        if key == __ESC_KEY:
            new_pos = __handle_escape_seq(buf, pos, stdscr)
            if isinstance(new_pos, UnknownEscapeSequence):
                continue

            pos = new_pos
            if pos is None:
                buf = None
                break
        elif key == __KEY_ENTER:
            if refuse_empty and len(buf) == 0:
                continue

            break
        elif key == curses.KEY_BACKSPACE or key == __KEY_BACKSPACE:
            if len(buf) > 0 and pos != 0:
                buf, pos = __remove_single_char(buf, pos, stdscr)

        elif key == __KEY_CTRLW:
            if len(buf) == 0 or pos == 0:
                continue
            elif buf[pos - 1] == " ":
                buf, pos = __remove_spaces_until_char(buf, pos, stdscr)

            buf, pos = __remove_chars_until_space(buf, pos, stdscr)
        elif key == __KEY_RIGHT:
            if pos < len(buf):
                pos += 1
                stdscr.move(1, pos + len(line_prompt))
        elif key == __KEY_LEFT:
            if pos > 0:
                pos -= 1
                stdscr.move(1, pos + len(line_prompt))
        elif key == __KEY_CTRL_LEFT:
            pos = __handle_ctrl_left(buf, pos)
        elif key == __KEY_CTRL_RIGHT:
            pos = __handle_ctrl_right(buf, pos)
        elif key == __KEY_CTRL_A:
            pos = 0
        elif key == __KEY_CTRL_E:
            pos = len(buf)
        elif __is_ascii(key):
            within_len_limit = len_limit is None or len(buf) < len_limit
            within_max_x = len(buf) + len(line_prompt) < max_x
            if within_len_limit and within_max_x:
                buf = buf[0:pos] + chr(key) + buf[pos:len(buf)]
                stdscr.addstr(chr(key))
                pos += 1
        else:
            continue

        if len_limit is not None:
            __draw_current_length(buf, len_limit, stdscr, len(line_prompt))

        __draw_buf(buf, pos, stdscr, max_x, len(line_prompt))
        stdscr.refresh()

    stdscr.clear()
    curses.set_escdelay(default_escdelay)
    curses.endwin()

    return buf
