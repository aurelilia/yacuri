mod memory;

use crate::{
    drivers::disk::{fat::FatFs, FileSystem},
    graphics::{draw_rect, Color},
    scheduling::task::Task,
};
pub use memory::init_code_heap;

pub fn test_app() {
    yacari::execute_path::<_, ()>(
        FileSystem::new(),
        &["test_app", "system/yacuri"],
        &[("draw_rect", test_draw_rect as *const u8)],
    )
    .unwrap();
}

fn test_draw_rect(x: i64, y: i64, w: i64, h: i64) {
    draw_rect(
        x as usize,
        y as usize,
        w as usize,
        h as usize,
        Color::from(81, 45, 168),
    )
}
