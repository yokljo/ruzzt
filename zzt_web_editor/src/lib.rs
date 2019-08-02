use zzt_file_format::{World, BoardTile};
use ruzzt_engine::console::{ConsoleColour, SCREEN_WIDTH, SCREEN_HEIGHT};
use ruzzt_engine::engine::RuzztEngine;
use num::FromPrimitive;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn zzt_to_json(zzt_data: &[u8]) -> Result<String, JsValue> {
	zzt_to_json_impl(zzt_data).map_err(|err| err.into())
}

pub fn zzt_to_json_impl(zzt_data: &[u8]) -> Result<String, String> {
	let mut cursor = std::io::Cursor::new(zzt_data);
	let world = World::parse(&mut cursor)?;
	let json_str = serde_json::to_string_pretty(&world).map_err(|e| format!("{:?}", e))?;
	Ok(json_str)
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct FgBgRgb {
	pub fg_r: u8,
	pub fg_g: u8,
	pub fg_b: u8,
	pub bg_r: u8,
	pub bg_g: u8,
	pub bg_b: u8,
	pub blinking: bool,
}

impl FgBgRgb {
	fn from_console_colours(fg: ConsoleColour, bg: ConsoleColour) -> FgBgRgb {
		let mut blinking = false;
		let mut back_num = bg as u8;
		if back_num >= 8 {
			back_num -= 8;
			blinking = true;
		}
	
		let real_bg = ruzzt_engine::console::ConsoleColour::from_u8(back_num).unwrap();
		let (fg_r, fg_g, fg_b) = fg.to_rgb();
		let (bg_r, bg_g, bg_b) = real_bg.to_rgb();
		FgBgRgb{fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, blinking}
	}
}

#[wasm_bindgen]
pub fn zzt_colour_to_rgb(zzt_colour: u8) -> FgBgRgb {
	let mut blinking = false;
	let mut bg_col = (zzt_colour & 0xF) >> 4;
	let fg_col = zzt_colour & 0xF;
	if bg_col >= 8 {
		bg_col -= 8;
		blinking = true;
	}
	
	let (fg_r, fg_g, fg_b) = ConsoleColour::from_u8(fg_col).unwrap().to_rgb();
	let (bg_r, bg_g, bg_b) = ConsoleColour::from_u8(bg_col).unwrap().to_rgb();
	FgBgRgb{fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, blinking}
}

#[wasm_bindgen]
pub struct ScreenChar {
	pub char_code: u8,
	pub colour: FgBgRgb,
}

#[wasm_bindgen]
struct WorldState {
	engine: RuzztEngine,
}

#[wasm_bindgen]
impl WorldState {
	pub fn from_file_data(zzt_file_data: &[u8]) -> Result<WorldState, JsValue> {
		Self::from_file_data_impl(zzt_file_data).map_err(|err| err.into())
	}
	
	fn from_file_data_impl(zzt_file_data: &[u8]) -> Result<WorldState, String> {
		let mut cursor = std::io::Cursor::new(zzt_file_data);
		let world = World::parse(&mut cursor)?;
		let mut engine = RuzztEngine::new();
		engine.load_world(world, None);
		engine.set_in_title_screen(false);
		
		Ok(WorldState {
			engine,
		})
	}
	
	pub fn get_world_json(&mut self) -> String {
		self.engine.sync_world();
		serde_json::to_string_pretty(&self.engine.world).unwrap()
	}
	
	pub fn get_world_header_json(&mut self) -> String {
		self.engine.sync_world();
		serde_json::to_string_pretty(&self.engine.world.world_header).unwrap()
	}
	
	pub fn get_current_board_index(&self) -> i16 {
		self.engine.world.world_header.player_board
	}
	
	pub fn get_status_elements_json(&mut self, board_index: i16) -> String {
		self.engine.sync_world();
		serde_json::to_string_pretty(&self.engine.world.boards[board_index as usize].status_elements).unwrap()
	}
	
	pub fn get_board_meta_data_json(&mut self, board_index: i16) -> String {
		self.engine.sync_world();
		serde_json::to_string_pretty(&self.engine.world.boards[board_index as usize].meta_data).unwrap()
	}
	
	pub fn get_tile_at(&mut self, x: i16, y: i16) -> String {
		serde_json::to_string_pretty(&self.engine.board_simulator.get_tile(x, y)).unwrap()
	}
	
	pub fn render_board(&mut self) -> js_sys::Array {
		let mut result_screen = js_sys::Array::new();
		self.engine.sync_world();
		self.engine.update_screen();
		let ref screen_chars = self.engine.console_state.screen_chars;
		for y in 0..SCREEN_HEIGHT {
			for x in 0..SCREEN_WIDTH {
				let ref c = screen_chars[y][x];
				let screen_char = ScreenChar {
					char_code: c.char_code,
					colour: FgBgRgb::from_console_colours(c.foreground, c.background),
				};
				result_screen.push(&JsValue::from(screen_char));
			}
		}
		result_screen
	}
}
