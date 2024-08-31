use win32console::console::WinConsole;
use win32console::input::*;

use rand::prelude::*;

// https://docs.rs/device_query/1.0.0/device_query/index.html
use device_query::{DeviceEvents, DeviceQuery, DeviceState, Keycode, MouseState};

use active_win_pos_rs::get_active_window;

use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use wincolor::{Console, Color, Intense};


const INITIAL_NUM_MINES: usize = 10;
const MAP_WIDTH: i16 = 9;
const MAP_HEIGHT: i16 = 9;

const FLAG_ID: u8 = 2u8;

fn main() {
    let mut con = Console::stdout().unwrap();
    
    WinConsole::output().clear().expect("Failed to clear the screen.");

    let mut bg_color = Color::Green; //Color::Magenta;

    con.bg(Intense::No, bg_color).unwrap();
    con.fg(Intense::Yes, Color::Red).unwrap();


    for y in 0..MAP_HEIGHT {
        let new_pos = Coord::new(0, y);
        WinConsole::output().set_cursor_position(new_pos).expect("Failed to set cursor");

        let background = "▢▢▢▢▢▢▢▢▢".encode_utf16().collect::<Vec<u16>>();
        WinConsole::output().write_utf16(background.as_slice()).expect("");
        
        bg_color = if bg_color == Color::Green {
            Color::Magenta
        } else {
            Color::Green
        };
        con.bg(Intense::No, bg_color).unwrap();
    }
    
    let mines_amount = AtomicI32::new(INITIAL_NUM_MINES as i32);
    draw_mines_panel(mines_amount.load(Ordering::Relaxed));

    let game_over = AtomicBool::new(false);

    let mut map: [[u8; 9]; 9] = [[0u8; 9]; 9];
    let open_cells_map: [[u8; 9]; 9] = [[0u8; 9]; 9];
    let open_cells_mutex: Arc<Mutex<[[u8; 9]; 9]>> = Arc::new(Mutex::new(open_cells_map));

    let mines_coords = generage_mines_coords();
    for (_, elem) in mines_coords.iter().enumerate() {
        let x = elem.0 as i16;
        let y = elem.1 as i16;

        map[x as usize][y as usize] = 1;
    }

    let device_state = DeviceState::new();
    let _guard = device_state.on_mouse_up(move |button| {
        if game_over.fetch_or(false, Ordering::Relaxed) {
            return;
        }        

        let device_state2 = DeviceState::new();
        let mouse: MouseState = device_state2.get_mouse();

        let active_window_res = get_active_window();
        let active_window = match active_window_res {
            Ok(active_window) => active_window,
            Err(_) => return
        };

        let window_pos_x = active_window.position.x as i32;
        let window_pos_y = active_window.position.y as i32;

        const BORDER_WIDTH: i32 = 8;
        const BORDER_HEIGHT: i32 = 40;

        let click_x = mouse.coords.0 - window_pos_x - BORDER_WIDTH;
        let click_y = mouse.coords.1 - window_pos_y - BORDER_HEIGHT;

        let symbol_pos_x: i16 = (click_x / 9) as i16 - 1;
        let symbol_pos_y: i16 = (click_y as f32 / 19.5) as i16;

        let is_outside_map = symbol_pos_x >= MAP_WIDTH || symbol_pos_y >= MAP_HEIGHT;
        if is_outside_map {
            return;
        }

        let is_already_opened = open_cells_map[symbol_pos_x as usize][symbol_pos_y as usize] == 1u8;
        if is_already_opened {
            return;
        }

        if *button == 2 {
            let x = symbol_pos_x as usize;
            let y = symbol_pos_y as usize;

            let mut open_cells_map_locked: MutexGuard<[[u8; 9]; 9]> = open_cells_mutex.lock().unwrap();
            let is_open = open_cells_map_locked[x][y] == 1;

            if is_open {
                return;
            }

            let need_install_flag = open_cells_map_locked[x][y] == 0;
            put_flag(symbol_pos_x, symbol_pos_y, need_install_flag);
            let added_mines = if need_install_flag { -1 } else { 1 };
            mines_amount.fetch_add(added_mines, Ordering::Relaxed);
            draw_mines_panel(mines_amount.load(Ordering::Relaxed));

            let flag_cell_value = if need_install_flag { 2 } else { 0 };
            open_cells_map_locked[x][y] = flag_cell_value;
            return;
        }

        let mut open_cells_map_locked: MutexGuard<[[u8; 9]; 9]> = open_cells_mutex.lock().unwrap();
        if is_bomb(map, symbol_pos_x, symbol_pos_y) && !is_flag(&mut open_cells_map_locked, symbol_pos_x, symbol_pos_y) {
            lose(&map);
            game_over.store(true, Ordering::Relaxed);
            return;
        }
        
        open_cell(symbol_pos_x as usize, symbol_pos_y as usize, map, &mut open_cells_map_locked, false);
        if check_win(map, open_cells_map_locked) {
            win();
            game_over.store(true, Ordering::Relaxed);
        }
     });

    loop {
    }
}

fn draw_mines_panel(num_mines: i32) {    
    let mut con = Console::stdout().unwrap();
    
    con.bg(Intense::No, Color::White).unwrap();
    con.fg(Intense::Yes, Color::Red).unwrap();

    let new_pos = Coord::new(0, 11);
    WinConsole::output().set_cursor_position(new_pos).expect("Failed to set cursor");

    let mut mines_info = "Mines: ".to_owned();
    mines_info.push_str(&to_str(num_mines));

    let background = "         ".encode_utf16().collect::<Vec<u16>>();
    WinConsole::output().write_utf16(background.as_slice()).expect("")
    ;
    WinConsole::output().set_cursor_position(new_pos).expect("Failed to set cursor");
    let background = mines_info.encode_utf16().collect::<Vec<u16>>();
    WinConsole::output().write_utf16(background.as_slice()).expect("");

    con.bg(Intense::No, Color::Green).unwrap();
    con.fg(Intense::Yes, Color::Red).unwrap();
}

fn lose(bomb_map: &[[u8; 9]; 9]) {
    let new_pos = Coord::new(0, 12);
    WinConsole::output().set_cursor_position(new_pos).expect("Failed to set position of the cursor");

    let text = "GAME OVER".encode_utf16().collect::<Vec<u16>>();
    WinConsole::output().write_utf16(text.as_slice()).expect("Failed to write the text");

    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            let x_u = x as usize;
            let y_u = y as usize;
            if bomb_map[x_u][y_u] == 1 {
                show_bomb(x, y);
            }
        }
    }
}

fn win() {
    let new_pos = Coord::new(0, 12);
    WinConsole::output().set_cursor_position(new_pos).expect("Failed to set position of the cursor");

    let text = "YOU WIN! 😎".encode_utf16().collect::<Vec<u16>>();
    WinConsole::output().write_utf16(text.as_slice()).expect("Failed to write the text");
}

fn check_win(bombs_map:[[u8; 9]; 9], open_cells_map: MutexGuard<[[u8; 9]; 9]>) -> bool {
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            let x_u = x as usize;
            let y_u = y as usize;
            if open_cells_map[x_u][y_u] == 1u8 { continue }
            if bombs_map[x_u][y_u] != 1 {
                return false
            }
        }
    }
    return true
}

fn open_cell(x: usize, y: usize, map:[[u8; 9]; 9], open_cells_map_locked: &mut MutexGuard<[[u8; 9]; 9]>, remove_flag: bool) {
    let is_already_opened = open_cells_map_locked[x][y] == 1;
    let is_flag = open_cells_map_locked[x][y] == 2;
    if is_already_opened {
        return;
    } 
    if is_flag && !remove_flag {
        return;
    }
    let num_mines_around = count_mines_around(map, x, y);
    show_number_mines(x, y, num_mines_around);


    open_cells_map_locked[x][y] = 1u8;
    
    let x: i16 = x as i16;
    let y: i16 = y as i16;
    if num_mines_around == 0 {
        for i in x-1..x+2 {
            for j in y-1..y+2 {
                if i >= 0 && i < MAP_WIDTH && j >= 0 && j < MAP_HEIGHT {
                    open_cell(i as usize, j as usize, map, open_cells_map_locked, true);
                }
            }
        }
    }
}

fn show_number_mines(x: usize, y: usize, num_mines: i32) {
    let new_pos = Coord::new(x as i16, y as i16);
    WinConsole::output().set_cursor_position(new_pos).expect("Failed to set cursor");

    let mut con = Console::stdout().unwrap();

    con.bg(Intense::No, Color::Blue).unwrap();
    con.fg(Intense::Yes, Color::Red).unwrap();

    let num_mines_str = if num_mines == 0 { String::from(" ") } else { to_str(num_mines) };

    let num_mines_utf16 = num_mines_str.encode_utf16().collect::<Vec<u16>>();
    WinConsole::output().write_utf16(num_mines_utf16.as_slice()).expect("Failed to write num the number of mines");
}

fn generage_mines_coords() -> [(u32, u32); INITIAL_NUM_MINES] {
    let mut all_coors: [u32; 81] = [0; 81];
    for (i, elem) in all_coors.iter_mut().enumerate() {
        *elem += i as u32;
    }

    let mut rng = rand::thread_rng();
    all_coors.shuffle(&mut rng);

    let mut coords: [(u32, u32); 10] = [(0,0); INITIAL_NUM_MINES];

    for i in 0..INITIAL_NUM_MINES {
        let elem = all_coors[i];
        let x = elem / 9;
        let y = elem % 9;
        coords[i] = (x, y);
    }
    return coords;
}

fn count_mines_around(map:[[u8; 9]; 9], x: usize, y: usize) -> i32 {
    use std::cmp;

    let x_left = cmp::max(0, (x as i32)-1);
    let x_right = cmp::min(8, (x as i32)+1) + 1;

    let y_top = cmp::max(0, (y as i32)-1);
    let y_bottom = cmp::min(8, (y as i32)+1) + 1;

    let mut count = 0;
    for i in x_left..x_right {
        for j in y_top..y_bottom {
            if map[i as usize][j as usize] == 1u8 {
                count += 1;
            }
        }
    }
    return count;
}

fn to_str(i: i32) -> String {
    format!("{}", i)
}

fn is_bomb(map:[[u8; 9]; 9], x: i16, y: i16) -> bool {
    return map[x as usize][y as usize] == 1u8;
}

fn is_flag(open_cells_map: &mut MutexGuard<[[u8; 9]; 9]>, x: i16, y: i16) -> bool {
    return open_cells_map[x as usize][y as usize] == FLAG_ID;
}

fn put_flag(x: i16, y: i16, set: bool) {
    let new_pos = Coord::new(x, y);
    WinConsole::output().set_cursor_position(new_pos).expect("Failed to set cursor position.");

    let flag_sign = if set { "F" } else { "▢" };
    
    let mut con = Console::stdout().unwrap();
    if set {
        con.bg(Intense::No, Color::Red).unwrap();
        con.fg(Intense::Yes, Color::Cyan).unwrap();
    } else {
        if y % 2 == 0 {
            con.bg(Intense::No, Color::Green).unwrap();
            con.fg(Intense::Yes, Color::Red).unwrap();
        } else {            
            con.bg(Intense::No, Color::Magenta).unwrap();
            con.fg(Intense::Yes, Color::Red).unwrap();
        }
    }
    let flag = flag_sign.encode_utf16().collect::<Vec<u16>>();
    WinConsole::output().write_utf16(flag.as_slice()).expect("Failed to put flag.");
}

fn show_bomb(x: i16, y: i16) {
    let new_pos = Coord::new(x, y);
    WinConsole::output().set_cursor_position(new_pos).expect("Failed to set cursor to bomb position.");
    let flag = "B".encode_utf16().collect::<Vec<u16>>();
    WinConsole::output().write_utf16(flag.as_slice()).expect("Failed to draw a bomb.");
}