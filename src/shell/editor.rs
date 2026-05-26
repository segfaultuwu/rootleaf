use crate::kernel::input;

const MAX_LINES: usize = 256;
const LINE_LEN: usize = 128;
const MAX_CMD: usize = 32;
const MAX_STATUS: usize = 96;
const MAX_SAVE: usize = 4096;
const VIEW_LINES: usize = 22;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

fn read_byte_poll() -> u8 {
    loop {
        if let Some(b) = input::dequeue() {
            return b;
        }

        crate::drivers::keyboard::poll_once();
        crate::kernel::present();

        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}

fn set_status(status: &mut [u8; MAX_STATUS], status_len: &mut usize, msg: &str) {
    let bytes = msg.as_bytes();
    let mut len = bytes.len();
    if len > MAX_STATUS {
        len = MAX_STATUS;
    }
    status[..len].copy_from_slice(&bytes[..len]);
    *status_len = len;
}

fn clear_screen_no_prompt() {
    let _ = crate::kernel::print::with_console(|console| {
        console.clear_screen();
    });
}

fn clamp_cursor(cursor_col: &mut usize, line_len: usize) {
    if *cursor_col > line_len {
        *cursor_col = line_len;
    }
}

fn render(
    name: &str,
    lines: &[[u8; LINE_LEN]; MAX_LINES],
    lens: &[usize; MAX_LINES],
    line_count: usize,
    cursor_line: usize,
    cursor_col: usize,
    mode: Mode,
    cmd: &[u8; MAX_CMD],
    cmd_len: usize,
    status: &[u8; MAX_STATUS],
    status_len: usize,
) {
    clear_screen_no_prompt();

    crate::kernel::write_raw("ROOTLEAF VIM  ");
    crate::kernel::write_raw(name);
    crate::kernel::write_raw("  -- ");
    match mode {
        Mode::Normal => crate::kernel::write_raw("NORMAL"),
        Mode::Insert => crate::kernel::write_raw("INSERT"),
        Mode::Command => crate::kernel::write_raw("COMMAND"),
    }
    crate::print!("\n");

    let top = if cursor_line >= VIEW_LINES {
        cursor_line - (VIEW_LINES - 1)
    } else {
        0
    };

    let end = core::cmp::min(top + VIEW_LINES, line_count);
    for i in top..end {
        let is_cur = i == cursor_line;
        if is_cur {
            crate::kernel::write_raw("> ");
        } else {
            crate::kernel::write_raw("  ");
        }

        let mut numbuf = [0u8; 20];
        crate::kernel::write_raw(crate::lib::u64_to_str((i + 1) as u64, &mut numbuf));
        crate::kernel::write_raw(" ");

        for c in 0..lens[i] {
            if is_cur && c == cursor_col {
                crate::kernel::write_byte(b'|');
            }
            crate::kernel::write_byte(lines[i][c]);
        }

        if is_cur && cursor_col == lens[i] {
            crate::kernel::write_byte(b'|');
        }

        crate::print!("\n");
    }

    for _ in end..(top + VIEW_LINES) {
        crate::kernel::write_raw("~\n");
    }

    crate::kernel::write_raw("-- ");
    if mode == Mode::Command {
        crate::kernel::write_byte(b':');
        for i in 0..cmd_len {
            crate::kernel::write_byte(cmd[i]);
        }
    } else {
        for i in 0..status_len {
            crate::kernel::write_byte(status[i]);
        }
    }

    crate::print!("\n");
    crate::kernel::present();
}

fn insert_empty_line(
    lines: &mut [[u8; LINE_LEN]; MAX_LINES],
    lens: &mut [usize; MAX_LINES],
    line_count: &mut usize,
    idx: usize,
) -> bool {
    if *line_count >= MAX_LINES || idx > *line_count {
        return false;
    }

    for i in (idx..*line_count).rev() {
        lines[i + 1] = lines[i];
        lens[i + 1] = lens[i];
    }

    lines[idx] = [0; LINE_LEN];
    lens[idx] = 0;
    *line_count += 1;
    true
}

fn delete_line(
    lines: &mut [[u8; LINE_LEN]; MAX_LINES],
    lens: &mut [usize; MAX_LINES],
    line_count: &mut usize,
    idx: usize,
) {
    if *line_count == 0 || idx >= *line_count {
        return;
    }

    for i in idx..(*line_count - 1) {
        lines[i] = lines[i + 1];
        lens[i] = lens[i + 1];
    }

    *line_count -= 1;

    if *line_count == 0 {
        lines[0] = [0; LINE_LEN];
        lens[0] = 0;
        *line_count = 1;
    }
}

fn save_script(
    path: &str,
    lines: &[[u8; LINE_LEN]; MAX_LINES],
    lens: &[usize; MAX_LINES],
    line_count: usize,
) -> bool {
    let mut out = [0u8; MAX_SAVE];
    let mut out_len = 0usize;

    for i in 0..line_count {
        if out_len + lens[i] + 1 > out.len() {
            return false;
        }

        out[out_len..out_len + lens[i]].copy_from_slice(&lines[i][..lens[i]]);
        out_len += lens[i];
        out[out_len] = b'\n';
        out_len += 1;
    }

    crate::fs::vfs::write(path, &out[..out_len]).is_ok()
}

pub fn launch(path: &str) {
    let mut lines: [[u8; LINE_LEN]; MAX_LINES] = [[0; LINE_LEN]; MAX_LINES];
    let mut lens: [usize; MAX_LINES] = [0; MAX_LINES];
    let mut line_count: usize = 1;

    if let Ok(data) = crate::fs::vfs::read(path) {
        if let Ok(text) = core::str::from_utf8(data) {
            let mut l = 0usize;
            let mut c = 0usize;

            for b in text.bytes() {
                if b == b'\n' {
                    lens[l] = c;
                    if l + 1 >= MAX_LINES {
                        break;
                    }
                    l += 1;
                    c = 0;
                    continue;
                }

                if c < LINE_LEN - 1 {
                    lines[l][c] = b;
                    c += 1;
                }
            }

            lens[l] = c;
            line_count = l + 1;
            if line_count == 0 {
                line_count = 1;
            }
        }
    }

    let mut mode = Mode::Normal;
    let mut cursor_line = 0usize;
    let mut cursor_col = 0usize;
    let mut cmd_buf = [0u8; MAX_CMD];
    let mut cmd_len = 0usize;
    let mut status = [0u8; MAX_STATUS];
    let mut status_len = 0usize;
    let mut pending_d = false;
    let mut dirty = false;

    set_status(
        &mut status,
        &mut status_len,
        "i:insert  h/j/k/l:move  x:del  o:newline  dd:del-line  :w :q :wq",
    );

    loop {
        clamp_cursor(&mut cursor_col, lens[cursor_line]);
        render(
            path,
            &lines,
            &lens,
            line_count,
            cursor_line,
            cursor_col,
            mode,
            &cmd_buf,
            cmd_len,
            &status,
            status_len,
        );

        let b = read_byte_poll();

        match mode {
            Mode::Insert => match b {
                0x1B => {
                    mode = Mode::Normal;
                    pending_d = false;
                    set_status(&mut status, &mut status_len, "-- NORMAL --");
                }

                b'\r' | b'\n' => {
                    if line_count < MAX_LINES {
                        let right_len = lens[cursor_line] - cursor_col;
                        let mut right = [0u8; LINE_LEN];
                        right[..right_len].copy_from_slice(
                            &lines[cursor_line][cursor_col..cursor_col + right_len],
                        );

                        lens[cursor_line] = cursor_col;

                        if insert_empty_line(
                            &mut lines,
                            &mut lens,
                            &mut line_count,
                            cursor_line + 1,
                        ) {
                            lines[cursor_line + 1][..right_len]
                                .copy_from_slice(&right[..right_len]);
                            lens[cursor_line + 1] = right_len;
                            cursor_line += 1;
                            cursor_col = 0;
                            dirty = true;
                        }
                    }
                }

                0x08 => {
                    if cursor_col > 0 {
                        for i in (cursor_col - 1)..(lens[cursor_line] - 1) {
                            lines[cursor_line][i] = lines[cursor_line][i + 1];
                        }
                        lens[cursor_line] -= 1;
                        cursor_col -= 1;
                        dirty = true;
                    } else if cursor_line > 0 {
                        let prev_len = lens[cursor_line - 1];
                        let cur_len = lens[cursor_line];

                        if prev_len + cur_len < LINE_LEN {
                            let mut tmp = [0u8; LINE_LEN];
                            tmp[..cur_len].copy_from_slice(&lines[cursor_line][..cur_len]);
                            lines[cursor_line - 1][prev_len..prev_len + cur_len]
                                .copy_from_slice(&tmp[..cur_len]);
                            lens[cursor_line - 1] += cur_len;
                            delete_line(&mut lines, &mut lens, &mut line_count, cursor_line);
                            cursor_line -= 1;
                            cursor_col = prev_len;
                            dirty = true;
                        }
                    }
                }

                ch => {
                    if ch < 0x20 || ch > 0x7e {
                        continue;
                    }

                    if lens[cursor_line] >= LINE_LEN - 1 {
                        continue;
                    }

                    for i in (cursor_col..lens[cursor_line]).rev() {
                        lines[cursor_line][i + 1] = lines[cursor_line][i];
                    }
                    lines[cursor_line][cursor_col] = ch;
                    lens[cursor_line] += 1;
                    cursor_col += 1;
                    dirty = true;
                }
            },

            Mode::Normal => match b {
                b'i' => {
                    mode = Mode::Insert;
                    pending_d = false;
                    set_status(&mut status, &mut status_len, "-- INSERT --");
                }

                b'a' => {
                    if cursor_col < lens[cursor_line] {
                        cursor_col += 1;
                    }
                    mode = Mode::Insert;
                    pending_d = false;
                    set_status(&mut status, &mut status_len, "-- INSERT --");
                }

                b'h' => {
                    pending_d = false;
                    cursor_col = cursor_col.saturating_sub(1);
                }

                b'l' => {
                    pending_d = false;
                    if cursor_col < lens[cursor_line] {
                        cursor_col += 1;
                    }
                }

                b'k' => {
                    pending_d = false;
                    if cursor_line > 0 {
                        cursor_line -= 1;
                    }
                }

                b'j' => {
                    pending_d = false;
                    if cursor_line + 1 < line_count {
                        cursor_line += 1;
                    }
                }

                b'0' => {
                    pending_d = false;
                    cursor_col = 0;
                }

                b'$' => {
                    pending_d = false;
                    cursor_col = lens[cursor_line];
                }

                b'x' => {
                    pending_d = false;
                    if cursor_col < lens[cursor_line] {
                        for i in cursor_col..(lens[cursor_line] - 1) {
                            lines[cursor_line][i] = lines[cursor_line][i + 1];
                        }
                        lens[cursor_line] -= 1;
                        dirty = true;
                    }
                }

                b'o' => {
                    pending_d = false;
                    if insert_empty_line(&mut lines, &mut lens, &mut line_count, cursor_line + 1) {
                        cursor_line += 1;
                        cursor_col = 0;
                        mode = Mode::Insert;
                        dirty = true;
                        set_status(&mut status, &mut status_len, "-- INSERT --");
                    }
                }

                b'd' => {
                    if pending_d {
                        delete_line(&mut lines, &mut lens, &mut line_count, cursor_line);
                        if cursor_line >= line_count {
                            cursor_line = line_count - 1;
                        }
                        cursor_col = 0;
                        dirty = true;
                        pending_d = false;
                    } else {
                        pending_d = true;
                        set_status(
                            &mut status,
                            &mut status_len,
                            "d pressed: press d again for dd",
                        );
                    }
                }

                b':' => {
                    pending_d = false;
                    mode = Mode::Command;
                    cmd_len = 0;
                }

                0x1B => {
                    pending_d = false;
                    set_status(&mut status, &mut status_len, "-- NORMAL --");
                }

                _ => {
                    pending_d = false;
                }
            },

            Mode::Command => match b {
                0x1B => {
                    mode = Mode::Normal;
                    cmd_len = 0;
                }

                0x08 => {
                    cmd_len = cmd_len.saturating_sub(1);
                }

                b'\r' | b'\n' => {
                    let cmd = &cmd_buf[..cmd_len];

                    if cmd == b"w" {
                        if save_script(path, &lines, &lens, line_count) {
                            dirty = false;
                            set_status(&mut status, &mut status_len, "written");
                        } else {
                            set_status(&mut status, &mut status_len, "write failed (size/slot)");
                        }
                        mode = Mode::Normal;
                    } else if cmd == b"q" {
                        if dirty {
                            set_status(
                                &mut status,
                                &mut status_len,
                                "no write since last change (:q! to force)",
                            );
                            mode = Mode::Normal;
                        } else {
                            clear_screen_no_prompt();
                            crate::kernel::prompt();
                            return;
                        }
                    } else if cmd == b"q!" {
                        clear_screen_no_prompt();
                        crate::kernel::prompt();
                        return;
                    } else if cmd == b"wq" {
                        if save_script(path, &lines, &lens, line_count) {
                            clear_screen_no_prompt();
                            crate::kernel::prompt();
                            return;
                        }
                        set_status(&mut status, &mut status_len, "write failed (size/slot)");
                        mode = Mode::Normal;
                    } else {
                        set_status(&mut status, &mut status_len, "unknown command");
                        mode = Mode::Normal;
                    }

                    cmd_len = 0;
                }

                ch => {
                    if ch >= 0x20 && ch <= 0x7e && cmd_len < MAX_CMD {
                        cmd_buf[cmd_len] = ch;
                        cmd_len += 1;
                    }
                }
            },
        }
    }
}
