//! A wlroots virtual pointer and keyboard driven from stdin, on one connection:
//! `move X Y`, `down BTN`, `up BTN`, `scroll DY`, `kdown CODE`, `kup CODE`, `mods MASK`.
//! Arguments: output width, height (pixels) and an XKB keymap file.
use std::io::BufRead;
use std::os::fd::AsFd;
use std::time::Instant;
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_pointer, wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

struct S;
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for S {
    fn event(_: &mut S, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<S>) {}
}
delegate_noop!(S: ignore wl_seat::WlSeat);
delegate_noop!(S: ZwlrVirtualPointerManagerV1);
delegate_noop!(S: ZwlrVirtualPointerV1);
delegate_noop!(S: ZwpVirtualKeyboardManagerV1);
delegate_noop!(S: ZwpVirtualKeyboardV1);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (w, h): (u32, u32) = (args[0].parse().unwrap(), args[1].parse().unwrap());
    let keymap = std::fs::File::open(&args[2]).expect("keymap");
    let size = keymap.metadata().unwrap().len() as u32;
    let conn = Connection::connect_to_env().expect("wayland");
    let (globals, mut queue) = registry_queue_init::<S>(&conn).expect("registry");
    let qh = queue.handle();
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=7, ()).expect("seat");
    let pointers: ZwlrVirtualPointerManagerV1 = globals.bind(&qh, 1..=2, ()).expect("virtual pointer manager");
    let keyboards: ZwpVirtualKeyboardManagerV1 = globals.bind(&qh, 1..=1, ()).expect("virtual keyboard manager");
    let pointer = pointers.create_virtual_pointer(Some(&seat), &qh, ());
    let keyboard = keyboards.create_virtual_keyboard(&seat, &qh, ());
    keyboard.keymap(1, keymap.as_fd(), size);
    queue.roundtrip(&mut S).unwrap();
    println!("ready");
    let start = Instant::now();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let p: Vec<&str> = line.split_whitespace().collect();
        let t = start.elapsed().as_millis() as u32;
        let num = |i: usize| p[i].parse::<u32>().unwrap();
        match p.first().copied() {
            Some("move") => {
                pointer.motion_absolute(t, num(1), num(2), w, h);
                pointer.frame();
            }
            Some("down") => {
                pointer.button(t, num(1), wl_pointer::ButtonState::Pressed);
                pointer.frame();
            }
            Some("up") => {
                pointer.button(t, num(1), wl_pointer::ButtonState::Released);
                pointer.frame();
            }
            Some("scroll") => {
                pointer.axis(t, wl_pointer::Axis::VerticalScroll, p[1].parse().unwrap());
                pointer.frame();
            }
            Some("kdown") => keyboard.key(t, num(1), 1),
            Some("kup") => keyboard.key(t, num(1), 0),
            Some("mods") => keyboard.modifiers(num(1), 0, 0, 0),
            Some("sync") => {
                queue.roundtrip(&mut S).unwrap();
                println!("ok");
                continue;
            }
            _ => continue,
        }
        conn.flush().unwrap();
        queue.dispatch_pending(&mut S).unwrap();
    }
    pointer.destroy();
    keyboard.destroy();
    conn.flush().ok();
}
