pub use crate::engine::RuzztEngine;
pub use crate::event::Event;
pub use crate::board_simulator::*;

pub use zzt_file_format::*;
pub use zzt_file_format::dosstring::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TestWorld {
	pub engine: RuzztEngine,
	pub event: Event,
}

impl TestWorld {
	pub fn new() -> TestWorld {
		// The player is at 29, 11, not including simulator borders (board edge tiles).
		let mut cursor = std::io::Cursor::new(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tests/data/DEFAULT.ZZT")).to_vec());
		let mut world = World::parse(&mut cursor).unwrap();
		// Remove the player.
		world.boards[1].status_elements.clear();
		world.boards[1].tiles[29 + 11*BOARD_WIDTH] = BoardTile::new(ElementType::Empty, 0);

		let mut engine = RuzztEngine::new();
		engine.load_world(world, None);
		engine.set_in_title_screen(false);
		engine.is_paused = false;
		
		TestWorld {
			engine,
			event: Event::None,
		}
	}
	
	pub fn new_with_player(x: i16, y: i16) -> TestWorld {
		let mut test_world = TestWorld::new();
		test_world.add_player(x, y);
		test_world
	}
	
	pub fn add_player(&mut self, x: i16, y: i16) {
		let mut tile_set = TileSet::new();
		tile_set.add('&', BoardTile::new(ElementType::Player, 0x1f), Some(StatusElement {
			cycle: 1,
			.. StatusElement::default()
		}));
		let player_template = TileTemplate::from_text(&tile_set, "&");
		self.insert_template(&player_template, x, y);
	}
	
	pub fn insert_tile_and_status(&mut self, tile_and_status: &TileAndStatus, x: i16, y: i16) {
		self.engine.board_simulator.set_tile(x, y, tile_and_status.tile);
		if let Some(ref status) = tile_and_status.status {
			let mut new_status = status.clone();
			new_status.location_x = x as u8;
			new_status.location_y = y as u8;
			self.engine.board_simulator.status_elements.push(new_status);
		}
	}
	
	pub fn insert_template(&mut self, template: &TileTemplate, left_x: i16, top_y: i16) {
		let mut it = template.tiles.iter();
		for y in 0 .. template.height as i16 {
			for x in 0 .. template.width as i16 {
				if let Some(tile_and_status) = it.next().as_mut().unwrap() {
					self.engine.board_simulator.set_tile(left_x + x, top_y + y, tile_and_status.tile);
					if let Some(ref status) = tile_and_status.status {
						let mut new_status = status.clone();
						new_status.location_x = (left_x + x) as u8;
						new_status.location_y = (top_y + y) as u8;
						self.engine.board_simulator.status_elements.push(new_status);
					}
				}
			}
		}
	}
	
	pub fn simulate(&mut self, step_count: usize) {
		for _ in 0 .. step_count {
			self.engine.step(self.event, 0.);
			self.event = Event::None;
		}
	}
	
	pub fn current_board_equals(&self, expected_world: TestWorld) -> bool {
		let mut result = true;
		
		let selfsim = &self.engine.board_simulator;
		let othersim = &expected_world.engine.board_simulator;
		if selfsim.world_header != othersim.world_header {
			println!("World headers differ");
			println!("Actual: {:?}", selfsim.world_header);
			println!("Expected: {:?}", othersim.world_header);
			result = false;
		}
		if selfsim.board_meta_data != othersim.board_meta_data {
			println!("Board meta data differs");
			println!("Actual: {:?}", selfsim.board_meta_data);
			println!("Expected: {:?}", othersim.board_meta_data);
			result = false;
		}
		if selfsim.status_elements != othersim.status_elements {
			println!("Status elements differ");
			println!("Actual: {:?}", selfsim.status_elements);
			println!("Expected: {:?}", othersim.status_elements);
			result = false;
		}
		
		result = result && self.current_board_tiles_equals(expected_world);
		
		result
	}
	
	pub fn current_board_tiles_equals(&self, expected_world: TestWorld) -> bool {
		let selfsim = &self.engine.board_simulator;
		let othersim = &expected_world.engine.board_simulator;
		if selfsim.tiles != othersim.tiles {
			let mut min_diff_x = BOARD_WIDTH as i16;
			let mut min_diff_y = BOARD_HEIGHT as i16;
			let mut max_diff_x = 0;
			let mut max_diff_y = 0;
			
			for x in 0 .. BOARD_WIDTH as i16 {
				for y in 0 .. BOARD_HEIGHT as i16 {
					let selftile = selfsim.get_tile(x, y).unwrap();
					let othertile = othersim.get_tile(x, y).unwrap();
					if selftile != othertile {
						max_diff_x = max_diff_x.max(x);
						max_diff_y = max_diff_y.max(y);
						min_diff_x = min_diff_x.min(x);
						min_diff_y = min_diff_y.min(y);
					}
				}
			}
			
			println!("Board differ from ({}, {}) to ({}, {}). Top lines are self, bottom lines are expected", min_diff_x, min_diff_y, max_diff_x, max_diff_y);
			for y in min_diff_y ..= max_diff_y {
				let mut self_line = "".to_string();
				let mut other_line = "".to_string();
				for x in min_diff_x ..= max_diff_x {
					let selftile = selfsim.get_tile(x, y).unwrap();
					let othertile = othersim.get_tile(x, y).unwrap();
					if selftile != othertile {
						self_line += &format!("{:02x},{:02x} ", selftile.element_id, selftile.colour);
						other_line += &format!("{:02x},{:02x} ", othertile.element_id, othertile.colour);
					} else {
						self_line += "==,== ";
						other_line += "==,== ";
					}
				}
				println!("{}", self_line);
				println!("{}", other_line);
				println!("");
			}
			
			false
		} else {
			true
		}
	}
	
	pub fn status_at(&mut self, x: i16, y: i16) -> &mut StatusElement {
		self.engine.board_simulator.get_first_status_for_pos_mut(x, y).unwrap().1
	}
	
	pub fn world_header(&self) -> &WorldHeader {
		&self.engine.board_simulator.world_header
	}
}

#[derive(Debug, Clone)]
pub struct TileAndStatus {
	pub tile: BoardTile,
	pub status: Option<StatusElement>,
}

pub struct TileSet {
	tile_map: HashMap<char, TileAndStatus>,
}

impl TileSet {
	pub fn new() -> TileSet {
		TileSet {
			tile_map: HashMap::new(),
		}
	}
	
	pub fn add(&mut self, c: char, tile: BoardTile, status: Option<StatusElement>) {
		self.tile_map.insert(c, TileAndStatus { tile, status });
	}
	
	pub fn add_object(&mut self, c: char, code: &str) {
		self.add(c, BoardTile::new(ElementType::Object, 0xff), Some(StatusElement {
			cycle: 1,
			code_source: CodeSource::Owned(DosString::from_str(code)),
			.. StatusElement::default()
		}));
	}
	
	pub fn get(&self, c: char) -> &TileAndStatus {
		self.tile_map.get(&c).ok_or_else(|| format!("TileSet::get: Tile not found for: {:?}", c)).unwrap()
	}
}

#[derive(Debug, Clone)]
pub struct TileTemplate {
	width: usize,
	height: usize,
	// Left-to-right, top-to-bottom order.
	tiles: Vec<Option<TileAndStatus>>,
}

impl TileTemplate {
	pub fn from_text(tile_set: &TileSet, text: &str) -> TileTemplate {
		let mut height = 0;
		let mut width = 0;
		let mut tiles = vec![];
		for line in text.lines() {
			let trimmed = line.trim().to_string();
			if !trimmed.is_empty() {
				let mut current_width = 0;
				for c in trimmed.chars() {
					if c == '.' {
						tiles.push(None);
					} else {
						tiles.push(Some(tile_set.get(c).clone()));
					}
					current_width += 1;
				}
				if width == 0 {
					width = current_width;
				} else if width != current_width {
					panic!("TileTemplate::from_text: Lines are inconsistent lengths");
				}
				height += 1;
			}
		}
		
		TileTemplate {
			width,
			height,
			tiles,
		}
	}
}
