// mod canvas_graphics;
// mod log;
// mod utils;

use std::cell::RefCell;
// use crate::canvas_graphics::{
//     Color, DefaultColor, Figure, FigureType, Point, Quaternion, Rasterizer, ScreenSetting,
// };
// use crate::log::{log, log_f64};
use rand::prelude::*;
use std::f64::consts::PI;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::Function;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, hello-wasm!");
}