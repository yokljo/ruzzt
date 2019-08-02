pub mod dosstring;

use crate::dosstring::DosString;

use serde_derive::{Serialize, Deserialize};
use num_derive::FromPrimitive;
#[allow(unused_imports)]
use num::FromPrimitive;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

// http://www.shikadi.net/moddingwiki/ZZT_Format
// http://zzt.org/zu/wiki/File_Format
// http://zzt.org/zzt/zztff.txt

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highscore {
	pub name: DosString,
	pub score: i16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highscores {
	pub scores: Vec<Highscore>,
}

impl Default for Highscores {
	fn default() -> Highscores {
		Highscores {
			scores: vec![],
		}
	}
}

impl Highscores {
	pub fn parse(stream: &mut std::io::Read) -> Result<Highscores, String> {
		let mut highscores = Highscores::default();
		for _ in 0 .. 30 {
			let name_len = stream.read_u8().map_err(|e| format!("Failed to read name length: {}", e))?;
			// NOTE: If the name_len is > 50, ZZT will just stop at 50.
			let mut name = DosString::new();
			for i in 0 .. 50 {
				let c = stream.read_u8().map_err(|e| format!("Failed to read name: {}", e))?;
				if i < name_len {
					name.push(c);
				}
			}
			let score = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read score: {}", e))?;
			if name_len > 0 {
				highscores.scores.push(Highscore{name, score});
			}
		}
		Ok(highscores)
	}

	pub fn write(&self, stream: &mut std::io::Write) -> Result<(), String> {
		for score_index in 0 .. 30 {
			if let Some(highscore) = self.scores.get(score_index) {
				let real_name_len = highscore.name.len().min(50) as u8;
				stream.write_u8(real_name_len).map_err(|e| format!("Failed to write name length: {}", e))?;
				for i in 0 .. 50 {
					let c = if i < highscore.name.len() {
						highscore.name.data[i]
					} else {
						0
					};
					stream.write_u8(c).map_err(|e| format!("Failed to write name: {}", e))?;
				}
				stream.write_i16::<LittleEndian>(highscore.score).map_err(|e| format!("Failed to write score: {}", e))?;
			} else {
				stream.write_u8(0).map_err(|e| format!("Failed to write dummy name length: {}", e))?;
				for _ in 0 .. 50 {
					stream.write_u8(0).map_err(|e| format!("Failed to write dummy name: {}", e))?;
				}
				stream.write_i16::<LittleEndian>(-1).map_err(|e| format!("Failed to write dummy score: {}", e))?;
			}
		}

		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
	pub world_header: WorldHeader,
	pub boards: Vec<Board>,
}

impl World {
	pub fn zzt_default() -> World {
		World {
			world_header: WorldHeader::zzt_default(),
			boards: vec![Board::zzt_default(DosString::from_slice(b"Title screen"))],
		}
	}

	pub fn parse<S: std::io::Read + std::io::Seek>(stream: &mut S) -> Result<World, String> {
		let world_header = WorldHeader::parse(stream).map_err(|e| format!("WorldHeader: {}", e))?;

		let board_offset = match world_header.world_type {
			WorldType::Zzt => 0x200,
			WorldType::SuperZzt => 0x400,
		};

		stream.seek(std::io::SeekFrom::Start(board_offset)).map_err(|e| format!("Failed to seek to {}: {}", board_offset, e))?;
		let mut boards = vec![];
		for _ in 0 .. (world_header.num_boards_except_title + 1) {
			let board = Board::parse(stream, world_header.world_type).map_err(|e| format!("Board: {}", e))?;
			boards.push(board);
		}

		Ok(World {
			world_header,
			boards,
		})
	}

	pub fn write(&self, stream: &mut std::io::Write) -> Result<(), String> {
		let mut header_buf = vec![];
		self.world_header.write(&mut header_buf).map_err(|e| format!("WorldHeader: {}", e))?;
		stream.write(&header_buf).map_err(|e| format!("Failed to write world header data: {}", e))?;

		let board_offset = match self.world_header.world_type {
			WorldType::Zzt => 0x200,
			WorldType::SuperZzt => 0x400,
		};

		let padding_count = board_offset - header_buf.len();

		for _ in 0 .. padding_count {
			stream.write_u8(0).map_err(|e| format!("Failed to write padding: {}", e))?;
		}

		for board in &self.boards {
			board.write(stream, self.world_header.world_type).map_err(|e| format!("Board: {}", e))?;
		}

		Ok(())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WorldType {
	/// *.ZZT
	Zzt,
	/// *.SZT
	SuperZzt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldHeader {
	pub world_type: WorldType,
	/// Add 1 to get the actual number of boards.
	pub num_boards_except_title: i16,
	pub player_ammo: i16,
	pub player_gems: i16,
	pub player_keys: [bool; 7],
	pub player_health: i16,
	pub player_board: i16,

	/// ZZT only.
	pub player_torches: Option<i16>,
	/// Game cycles left for a torch to remain lit. ZZT only.
	pub torch_cycles: Option<i16>,
	/// Game cycles left for an energiser to take effect.
	pub energy_cycles: i16,
	pub player_score: i16,
	pub world_name: DosString,
	/// ZZT has 10 flags, SZT has 16.
	pub flag_names: Vec<DosString>,
	/// Amount of time passed in seconds on this board, if the player is on a time limit.
	pub time_passed: i16,
	/// This value is changed approximately every second by adding the number of hundredths of
	/// a second since the last time it changed its value. The value is modulus 6000, which is the
	/// number of centiseconds in a minute.
	pub time_passed_ticks: i16,
	pub locked: bool,
	/// SZT only.
	pub player_stones: Option<i16>,
}

impl WorldHeader {
	pub fn zzt_default() -> WorldHeader {
		WorldHeader {
			world_type: WorldType::Zzt,
			num_boards_except_title: 0,
			player_ammo: 0,
			player_gems: 0,
			player_keys: [false; 7],
			player_health: 100,
			player_board: 0,
			player_torches: Some(0),
			torch_cycles: Some(0),
			energy_cycles: 0,
			player_score: 0,
			world_name: DosString::from_slice(b""),
			flag_names: vec![DosString::from_slice(b""); 10],
			time_passed: 0,
			time_passed_ticks: 0,
			locked: false,
			player_stones: None,
		}
	}

	pub fn parse(stream: &mut std::io::Read) -> Result<WorldHeader, String> {
		let world_type_num = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read world type: {}", e))?;
		let world_type = match world_type_num {
			-1 => WorldType::Zzt,
			-2 => WorldType::SuperZzt,
			_ => return Err(format!("Invalid world type: {}", world_type_num)),
		};

		let num_boards_except_title = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read number of boards: {}", e))?;

		let player_ammo = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player's ammo count: {}", e))?;
		let player_gems = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player's gem count: {}", e))?;

		let mut player_keys = [false; 7];
		for key_index in 0 .. 7 {
			let key_state = stream.read_u8().map_err(|e| format!("Failed to read player's key states: {}", e))?;
			player_keys[key_index] = key_state > 0;
		}

		let player_health = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player's health: {}", e))?;

		let player_board = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player's current board index: {}", e))?;

		let (player_torches, torch_cycles) = match world_type {
			WorldType::Zzt => {
				let player_torches = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player torches: {}", e))?;
				let torch_cycles = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read torch cycles: {}", e))?;

				(Some(player_torches), Some(torch_cycles))
			}
			WorldType::SuperZzt => {
				let _padding = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read padding: {}", e))?;
				(None, None)
			}
		};

		let (energy_cycles, player_score) = match world_type {
			WorldType::Zzt => {
				let energy_cycles = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read energy cycles: {}", e))?;
				let _padding = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read padding: {}", e))?;
				let player_score = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player score: {}", e))?;
				(energy_cycles, player_score)
			}
			WorldType::SuperZzt => {
				let player_score = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player score: {}", e))?;
				let _padding = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read padding: {}", e))?;
				let energy_cycles = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read energy cycles: {}", e))?;
				(energy_cycles, player_score)
			}
		};

		let world_name_len = stream.read_u8().map_err(|e| format!("Failed to read world name length: {}", e))?;
		let mut world_name = DosString::new();
		for i in 0 .. 20 {
			let c = stream.read_u8().map_err(|e| format!("Failed to read world name: {}", e))?;
			if i < world_name_len {
				world_name.push(c);
			}
		}

		let mut flag_names = vec![];
		let flag_names_count = match world_type {
			WorldType::Zzt => 10,
			WorldType::SuperZzt => 16,
		};
		for _ in 0 .. flag_names_count {
			let flag_name_len = stream.read_u8().map_err(|e| format!("Failed to read flag name length: {}", e))?;
			let mut flag_name = DosString::new();
			for i in 0 .. 20 {
				let c = stream.read_u8().map_err(|e| format!("Failed to read flag name: {}", e))?;
				if i < flag_name_len {
					flag_name.push(c);
				}
			}
			flag_names.push(flag_name);
		}

		let time_passed = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read time passed: {}", e))?;
		let time_passed_ticks = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read time passed ticks: {}", e))?;
		let locked_num = stream.read_u8().map_err(|e| format!("Failed to read locked: {}", e))?;
		let locked = locked_num == 0;

		let player_stones = match world_type {
			WorldType::Zzt => {
				for _ in 0 .. 14 {
					stream.read_u8().map_err(|e| format!("Failed to read padding bytes: {}", e))?;
				}
				None
			}
			WorldType::SuperZzt => {
				let player_stones = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read player stones: {}", e))?;

				for _ in 0 .. 11 {
					stream.read_u8().map_err(|e| format!("Failed to read padding bytes: {}", e))?;
				}
				Some(player_stones)
			}
		};

		Ok(WorldHeader {
			world_type,
			num_boards_except_title,
			player_ammo,
			player_gems,
			player_keys,
			player_health,
			player_board,
			player_torches,
			torch_cycles,
			energy_cycles,
			player_score,
			world_name,
			flag_names,
			time_passed,
			time_passed_ticks,
			locked,
			player_stones,
		})
	}

	fn write(&self, stream: &mut std::io::Write) -> Result<(), String> {
		let world_type_num = match self.world_type {
			WorldType::Zzt => -1,
			WorldType::SuperZzt => -2,
		};
		stream.write_i16::<LittleEndian>(world_type_num).map_err(|e| format!("Failed to write world type: {}", e))?;

		stream.write_i16::<LittleEndian>(self.num_boards_except_title).map_err(|e| format!("Failed to write number of boards: {}", e))?;

		stream.write_i16::<LittleEndian>(self.player_ammo).map_err(|e| format!("Failed to write player's ammo count: {}", e))?;
		stream.write_i16::<LittleEndian>(self.player_gems).map_err(|e| format!("Failed to write player's gem count: {}", e))?;

		for key_state in &self.player_keys {
			stream.write_u8(if *key_state {1} else {0}).map_err(|e| format!("Failed to write player's key states: {}", e))?;
		}

		stream.write_i16::<LittleEndian>(self.player_health).map_err(|e| format!("Failed to write player's health: {}", e))?;

		stream.write_i16::<LittleEndian>(self.player_board).map_err(|e| format!("Failed to write player's current board index: {}", e))?;

		match self.world_type {
			WorldType::Zzt => {
				let player_torches = self.player_torches.ok_or_else(|| format!("Can't write player torches: not set"))?;
				let torch_cycles = self.torch_cycles.ok_or_else(|| format!("Can't write torch cycles: not set"))?;
				stream.write_i16::<LittleEndian>(player_torches).map_err(|e| format!("Failed to write player torches: {}", e))?;
				stream.write_i16::<LittleEndian>(torch_cycles).map_err(|e| format!("Failed to write torch cycles: {}", e))?;
			}
			WorldType::SuperZzt => {
				stream.write_i16::<LittleEndian>(0).map_err(|e| format!("Failed to write padding: {}", e))?;
			}
		};

		match self.world_type {
			WorldType::Zzt => {
				stream.write_i16::<LittleEndian>(self.energy_cycles).map_err(|e| format!("Failed to write energy cycles: {}", e))?;
				stream.write_i16::<LittleEndian>(0).map_err(|e| format!("Failed to write padding: {}", e))?;
				stream.write_i16::<LittleEndian>(self.player_score).map_err(|e| format!("Failed to write player score: {}", e))?;
			}
			WorldType::SuperZzt => {
				stream.write_i16::<LittleEndian>(self.player_score).map_err(|e| format!("Failed to write player score: {}", e))?;
				stream.write_i16::<LittleEndian>(0).map_err(|e| format!("Failed to write padding: {}", e))?;
				stream.write_i16::<LittleEndian>(self.energy_cycles).map_err(|e| format!("Failed to write energy cycles: {}", e))?;
			}
		}

		stream.write_u8(self.world_name.len() as u8).map_err(|e| format!("Failed to write world name length: {}", e))?;
		for i in 0 .. 20 {
			let c = if i < self.world_name.len() {
				self.world_name.data[i]
			} else {
				0
			};
			stream.write_u8(c).map_err(|e| format!("Failed to write world name: {}", e))?;
		}

		let flag_names_count = match self.world_type {
			WorldType::Zzt => 10,
			WorldType::SuperZzt => 16,
		};

		if self.flag_names.len() != flag_names_count {
			return Err(format!("Wrong number of flags: {} (expected {})", self.flag_names.len(), flag_names_count));
		}

		for flag_name in &self.flag_names {
			stream.write_u8(flag_name.len() as u8).map_err(|e| format!("Failed to write flag name length: {}", e))?;
			for i in 0 .. 20 {
				let c = if i < flag_name.len() {
					flag_name.data[i]
				} else {
					0
				};
				stream.write_u8(c).map_err(|e| format!("Failed to write flag name: {}", e))?;
			}
		}

		stream.write_i16::<LittleEndian>(self.time_passed).map_err(|e| format!("Failed to write time passed: {}", e))?;
		stream.write_i16::<LittleEndian>(self.time_passed_ticks).map_err(|e| format!("Failed to write time passed ticks: {}", e))?;
		stream.write_u8(if self.locked {0} else {1}).map_err(|e| format!("Failed to write locked: {}", e))?;

		match self.world_type {
			WorldType::Zzt => {
				for _ in 0 .. 14 {
					stream.write_u8(0).map_err(|e| format!("Failed to write padding bytes: {}", e))?;
				}
			}
			WorldType::SuperZzt => {
				let player_stones = self.player_stones.ok_or_else(|| format!("Can't write player stones: not set"))?;
				stream.write_i16::<LittleEndian>(player_stones).map_err(|e| format!("Failed to write player stones: {}", e))?;

				for _ in 0 .. 11 {
					stream.write_u8(0).map_err(|e| format!("Failed to write padding bytes: {}", e))?;
				}
			}
		};

		Ok(())
	}

	pub fn first_empty_flag(&self) -> Option<usize> {
		for (index, flag_name) in self.flag_names.iter().enumerate() {
			if flag_name.is_empty() {
				return Some(index);
			}
		}
		None
	}

	pub fn last_matching_flag(&self, check_flag_name: DosString) -> Option<usize> {
		let check_flag_name = check_flag_name.to_upper();
		for (index, flag_name) in self.flag_names.iter().enumerate().rev() {
			if check_flag_name == *flag_name {
				// A flag is "set" if it is in the flags list. The last instance is returned if
				// there are multiple, hence the .rev().
				return Some(index);
			}
		}
		None
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[derive(FromPrimitive)]
#[repr(u8)]
pub enum ElementType {
	Empty = 0,
	BoardEdge,
	Messenger,
	Monitor,
	Player,
	Ammo,
	Torch,
	Gem,
	Key,
	Door,
	Scroll,
	Passage,
	Duplicator,
	Bomb,
	Energizer,
	Star,
	Clockwise,
	Counter,
	Bullet,
	Water,
	Forest,
	Solid,
	Normal,
	Breakable,
	Boulder,
	SliderNS,
	SliderEW,
	Fake,
	Invisible,
	BlinkWall,
	Transporter,
	Line,
	Ricochet,
	BlinkRayHorizontal,
	Bear,
	Ruffian,
	Object,
	Slime,
	Shark,
	SpinningGun,
	Pusher,
	Lion,
	Tiger,
	BlinkRayVertical,
	Head,
	Segment = 45,
	TextBlue = 47,
	TextGreen,
	TextCyan,
	TextRed,
	TextPurple,
	TextBrown,
	TextBlack,
}

/// Turn element IDs into strings that are either an entry from ElementType, or a stringified number
/// if there is no corresponding entry in the enum.
mod element_id_serde {
	use super::*;
	use serde::{de, Serialize, Deserialize, Serializer, Deserializer};
	use serde::de::value::StrDeserializer;
	use serde::de::IntoDeserializer;

	pub fn serialize<S>(element_id: &u8, serializer: S) -> Result<S::Ok, S::Error> where
		S: Serializer
	{
		if let Some(element_type) = ElementType::from_u8(*element_id) {
			element_type.serialize(serializer)
		} else {
			serializer.serialize_str(&format!("{}", element_id))
		}
	}

	struct ElementIdVisitor;

	impl<'de> de::Visitor<'de> for ElementIdVisitor {
		type Value = u8;

		fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
			formatter.write_str("an element type (by name or by u8 ID)")
		}

		fn visit_u64<E>(self, value: u64) -> Result<u8, E> where
			E: de::Error,
		{
			if value > u8::max_value() as u64 {
				Err(E::custom(format!("Element ID too big: {}", value)))
			} else {
				Ok(value as u8)
			}
		}

		fn visit_str<E>(self, value: &str) -> Result<u8, E> where
			E: de::Error,
		{
			let element_type_de: StrDeserializer<E> = value.into_deserializer();
			if let Ok(val) = ElementType::deserialize(element_type_de) {
				return Ok(val as u8);
			}
			value.parse::<u8>().map_err(|e| E::custom(format!("{:?}", e)))
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<u8, D::Error> where
		D: Deserializer<'de>
	{
		deserializer.deserialize_any(ElementIdVisitor)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoardTile {
	#[serde(with = "element_id_serde")]
	pub element_id: u8,
	pub colour: u8,
}

impl BoardTile {
	pub fn new(element_type: ElementType, colour: u8) -> BoardTile {
		BoardTile{element_id: element_type as u8, colour}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardMetaData {
	pub board_name: DosString,
	pub max_player_shots: u8,
	pub is_dark: bool,
	pub exit_north: u8,
	pub exit_south: u8,
	pub exit_west: u8,
	pub exit_east: u8,
	pub restart_on_zap: bool,
	pub message: Option<DosString>,
	pub player_enter_x: u8,
	pub player_enter_y: u8,
	pub camera_x: Option<i16>,
	pub camera_y: Option<i16>,
	/// The time limit of the board, in seconds.
	pub time_limit: i16,
}

impl Default for BoardMetaData {
	fn default() -> BoardMetaData {
		BoardMetaData {
			board_name: DosString::from_str("Default Board"),
			max_player_shots: 255,
			is_dark: false,
			exit_north: 0,
			exit_south: 0,
			exit_west: 0,
			exit_east: 0,
			restart_on_zap: false,
			message: None,
			player_enter_x: 0,
			player_enter_y: 0,
			camera_x: None,
			camera_y: None,
			time_limit: 0,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board {
	/// ZZT: 60x25, SZT: 96x80
	pub tiles: Vec<BoardTile>,
	pub status_elements: Vec<StatusElement>,
	pub meta_data: BoardMetaData,
}

impl Default for Board {
	fn default() -> Board {
		let mut tiles = vec![];
		for _ in 0 .. (25 * 60) {
			tiles.push(BoardTile {
				element_id: 0,
				colour: 0,
			});
		}

		Board {
			tiles,
			status_elements: vec![],
			meta_data: BoardMetaData::default(),
		}
	}
}

impl Board {
	pub fn zzt_default(name: DosString) -> Board {
		let mut board = Board::default();
		board.meta_data.board_name = name;

		board.status_elements.push(StatusElement {
			location_x: 30,
			location_y: 12,
			.. StatusElement::default()
		});
		board.tiles[29 + 60*11] = BoardTile {
			element_id: ElementType::Player as u8,
			colour: 0x1f,
		};

		let border_tile = BoardTile {
			element_id: ElementType::Normal as u8,
			colour: 0x0e,
		};
		// Add the awful yellow border:
		for x in 0..60 {
			board.tiles[x] = border_tile;
			board.tiles[x + (24 * 60)] = border_tile;
		}
		for y in 1..24 {
			board.tiles[y * 60] = border_tile;
			board.tiles[y * 60 + 59] = border_tile;
		}
		board
	}

	pub fn parse(stream: &mut std::io::Read, world_type: WorldType) -> Result<Board, String> {
		// Board header:
		let board_size = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read board size: {}", e))?;
		let board_name_len = stream.read_u8().map_err(|e| format!("Failed to read board name length: {}", e))?;
		let mut board_name = DosString::new();
		let max_board_name_bytes = match world_type {
			WorldType::Zzt => 50,
			WorldType::SuperZzt => 60,
		};
		for i in 0 .. max_board_name_bytes {
			let c = stream.read_u8().map_err(|e| format!("Failed to read board name: {}", e))?;
			if i < board_name_len {
				board_name.push(c);
			}
		}

		if board_size < 0 {
			return Err("Board size can't be less than 0".into());
		}

		let tile_count = match world_type {
			WorldType::Zzt => 60 * 25,
			WorldType::SuperZzt => 96 * 80,
		};

		// Run-length-encoded tile data:
		let mut tiles = vec![];
		while tiles.len() < tile_count {
			let mut run_length = stream.read_u8().map_err(|e| format!("Failed to read tile run length: {}", e))? as usize;
			if run_length == 0 {
				run_length = 256;
			}

			let element_id = stream.read_u8().map_err(|e| format!("Failed to read tile element ID: {}", e))?;
			let colour = stream.read_u8().map_err(|e| format!("Failed to read tile colour: {}", e))?;
			for _ in 0..run_length {
				tiles.push(BoardTile{element_id, colour});
			}
		}

		// Board properties:

		let max_player_shots = stream.read_u8().map_err(|e| format!("Failed to read max player shots: {}", e))?;

		let is_dark = match world_type {
			WorldType::Zzt => {
				let is_dark_num = stream.read_u8().map_err(|e| format!("Failed to read is dark: {}", e))?;

				is_dark_num > 0
			}
			WorldType::SuperZzt => {
				false
			}
		};

		let exit_north = stream.read_u8().map_err(|e| format!("Failed to read north exit: {}", e))?;
		let exit_south = stream.read_u8().map_err(|e| format!("Failed to read south exit: {}", e))?;
		let exit_west = stream.read_u8().map_err(|e| format!("Failed to read west exit: {}", e))?;
		let exit_east = stream.read_u8().map_err(|e| format!("Failed to read east exit: {}", e))?;
		let restart_on_zap_num = stream.read_u8().map_err(|e| format!("Failed to read restart on zap: {}", e))?;
		let restart_on_zap = restart_on_zap_num == 1;

		let message = match world_type {
			WorldType::Zzt => {
				let message_len = stream.read_u8().map_err(|e| format!("Failed to read message length: {}", e))?;
				let mut message = DosString::new();
				for i in 0 .. 58 {
					let c = stream.read_u8().map_err(|e| format!("Failed to read message: {}", e))?;
					if i < message_len {
						message.push(c);
					}
				}
				Some(message)
			}
			WorldType::SuperZzt => {
				None
			}
		};

		let player_enter_x = stream.read_u8().map_err(|e| format!("Failed to read player enter X: {}", e))?;
		let player_enter_y = stream.read_u8().map_err(|e| format!("Failed to read player enter Y: {}", e))?;

		let camera_x = match world_type {
			WorldType::Zzt => {
				None
			}
			WorldType::SuperZzt => {
				Some(stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read camera X: {}", e))?)
			}
		};

		let camera_y = match world_type {
			WorldType::Zzt => {
				None
			}
			WorldType::SuperZzt => {
				Some(stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read camera Y: {}", e))?)
			}
		};

		let time_limit = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read time limit: {}", e))?;

		let padding_byte_count = match world_type {
			WorldType::Zzt => 16,
			WorldType::SuperZzt => 14,
		};
		for _ in 0 .. padding_byte_count {
			let _padding_byte = stream.read_u8().map_err(|e| format!("Failed to read padding bytes: {}", e))?;
		}

		let stat_element_count_minus_one = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read status element count: {}", e))?;

		let mut status_elements = vec![];
		for _ in 0 .. (stat_element_count_minus_one + 1) {
			let status_element = StatusElement::parse(stream, world_type).map_err(|e| format!("StatusElement: {}", e))?;
			status_elements.push(status_element);
		}

		Ok(Board {
			tiles,
			status_elements,
			meta_data: BoardMetaData {
				board_name,
				max_player_shots,
				is_dark,
				exit_north,
				exit_south,
				exit_west,
				exit_east,
				restart_on_zap,
				message,
				player_enter_x,
				player_enter_y,
				camera_x,
				camera_y,
				time_limit,
			}
		})
	}

	fn write(&self, final_stream: &mut std::io::Write, world_type: WorldType) -> Result<(), String> {
		// Need to buffer the whole board before writing it so the board_size can be calculated then
		// written out first:
		let mut stream = vec![];

		stream.write_u8(self.meta_data.board_name.len() as u8).map_err(|e| format!("Failed to write board name length: {}", e))?;

		let max_board_name_bytes = match world_type {
			WorldType::Zzt => 50,
			WorldType::SuperZzt => 60,
		};
		for i in 0 .. max_board_name_bytes {
			let c = if i < self.meta_data.board_name.len() {
				self.meta_data.board_name.data[i]
			} else {
				0
			};
			stream.write_u8(c).map_err(|e| format!("Failed to write board name: {}", e))?;
		}

		let tile_count = match world_type {
			WorldType::Zzt => 60 * 25,
			WorldType::SuperZzt => 96 * 80,
		};

		if self.tiles.len() != tile_count {
			return Err(format!("Wrong number of tiles: {} (expected {})", self.tiles.len(), tile_count));
		}

		// Run-length-encoded tile data:
		{
			let mut write_tile_run = |tile: BoardTile, run_len: usize| {
				let len_to_write = if run_len > 256 {
					return Err(format!("Attempted to write run length > 256"));
				} else if run_len == 256 {
					0
				} else {
					run_len as u8
				};

				stream.write_u8(len_to_write).map_err(|e| format!("Failed to write tile run length: {}", e))?;
				stream.write_u8(tile.element_id).map_err(|e| format!("Failed to write tile element ID: {}", e))?;
				stream.write_u8(tile.colour).map_err(|e| format!("Failed to write tile colour: {}", e))?;

				Ok(())
			};

			let mut current_run_len = 0;
			let mut working_tile = None;

			for tile in &self.tiles {
				if let Some(last_tile) = working_tile {
					if last_tile != *tile {
						write_tile_run(last_tile, current_run_len)?;
						current_run_len = 0;
					}
				}

				current_run_len += 1;

				if current_run_len == 256 {
					write_tile_run(working_tile.unwrap(), current_run_len)?;
					current_run_len = 0;
				}
				working_tile = Some(*tile);
			}

			if current_run_len > 0 {
				write_tile_run(working_tile.unwrap(), current_run_len)?;
			}
		}

		// Board properties:

		stream.write_u8(self.meta_data.max_player_shots).map_err(|e| format!("Failed to write max player shots: {}", e))?;

		match world_type {
			WorldType::Zzt => {
				stream.write_u8(if self.meta_data.is_dark {1} else {0}).map_err(|e| format!("Failed to write is dark: {}", e))?;
			}
			WorldType::SuperZzt => {}
		};

		stream.write_u8(self.meta_data.exit_north).map_err(|e| format!("Failed to write north exit: {}", e))?;
		stream.write_u8(self.meta_data.exit_south).map_err(|e| format!("Failed to write south exit: {}", e))?;
		stream.write_u8(self.meta_data.exit_west).map_err(|e| format!("Failed to write west exit: {}", e))?;
		stream.write_u8(self.meta_data.exit_east).map_err(|e| format!("Failed to write east exit: {}", e))?;
		stream.write_u8(if self.meta_data.restart_on_zap {1} else {0}).map_err(|e| format!("Failed to write restart on zap: {}", e))?;

		match world_type {
			WorldType::Zzt => {
				let message = self.meta_data.message.as_ref().ok_or_else(|| format!("Can't write message: not set"))?;

				stream.write_u8(message.len() as u8).map_err(|e| format!("Failed to write world name length: {}", e))?;
				for i in 0 .. 58 {
					let c = if i < message.len() {
						message.data[i]
					} else {
						0
					};
					stream.write_u8(c).map_err(|e| format!("Failed to write message: {}", e))?;
				}
			}
			WorldType::SuperZzt => {}
		}

		stream.write_u8(self.meta_data.player_enter_x).map_err(|e| format!("Failed to write player enter X: {}", e))?;
		stream.write_u8(self.meta_data.player_enter_y).map_err(|e| format!("Failed to write player enter Y: {}", e))?;

		match world_type {
			WorldType::Zzt => {}
			WorldType::SuperZzt => {
				let camera_x = self.meta_data.camera_x.ok_or_else(|| format!("Can't write camera X: not set"))?;
				stream.write_i16::<LittleEndian>(camera_x).map_err(|e| format!("Failed to write camera X: {}", e))?;
			}
		};

		match world_type {
			WorldType::Zzt => {}
			WorldType::SuperZzt => {
				let camera_y = self.meta_data.camera_y.ok_or_else(|| format!("Can't write camera Y: not set"))?;
				stream.write_i16::<LittleEndian>(camera_y).map_err(|e| format!("Failed to write camera Y: {}", e))?;
			}
		};

		stream.write_i16::<LittleEndian>(self.meta_data.time_limit).map_err(|e| format!("Failed to write time limit: {}", e))?;

		let padding_byte_count = match world_type {
			WorldType::Zzt => 16,
			WorldType::SuperZzt => 14,
		};
		for _ in 0 .. padding_byte_count {
			stream.write_u8(0).map_err(|e| format!("Failed to write padding bytes: {}", e))?;
		}

		if self.status_elements.len() < 1 {
			return Err(format!("Can't have less than 1 status element"));
		} else if self.status_elements.len() > i16::max_value() as usize {
			return Err(format!("Can't have more than than {} status elements", i16::max_value()));
		}

		stream.write_i16::<LittleEndian>((self.status_elements.len() - 1) as i16).map_err(|e| format!("Failed to write status element count: {}", e))?;

		for status_element in &self.status_elements {
			status_element.write(&mut stream, world_type).map_err(|e| format!("StatusElement: {}", e))?;
		}

		// Now write out the board size and content:

		if self.status_elements.len() > i16::max_value() as usize {
			return Err(format!("Can't have board size greater than than than {}", i16::max_value()));
		}

		final_stream.write_i16::<LittleEndian>(stream.len() as i16).map_err(|e| format!("Failed to write board size: {}", e))?;
		final_stream.write(&stream).map_err(|e| format!("Failed to write board data: {}", e))?;

		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodeSource {
	Owned(DosString),
	Bound(usize),
}

impl CodeSource {
	fn get_save_code_length(&self) -> i16 {
		match self {
			CodeSource::Owned(code) => code.len() as i16,
			CodeSource::Bound(bound_index) => -(*bound_index as i16),
		}
	}
}

/// Status elements point at a tile on the board and apply active simulation to it. Basically on
/// each simulation step, iterate through all the status elements and update accordingly, then the
/// simulation step is complete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusElement {
	/// This number is 1-based relative to the board's tiles because the simulator adds a border of
	/// BoardEdge tiles around the board before simulating it.
	pub location_x: u8,
	/// This number is 1-based relative to the board's tiles because the simulator adds a border of
	/// BoardEdge tiles around the board before simulating it.
	pub location_y: u8,
	pub step_x: i16,
	pub step_y: i16,
	pub cycle: i16,
	/// For Objects, this is the character code that they are drawn on the screen with.
	pub param1: u8,
	/// For Objects, this is 1 when they are locked.
	pub param2: u8,
	pub param3: u8,
	pub follower: i16,
	/// This is -1 when there is no leader, and -2 when it is a segment that is about to become a
	/// head.
	pub leader: i16,
	#[serde(with = "element_id_serde")]
	pub under_element_id: u8,
	pub under_colour: u8,
	// This becomes -1 when an error returns, so the program stops running.
	pub code_current_instruction: i16,
	pub code_source: CodeSource,
}

impl StatusElement {
	fn parse(stream: &mut std::io::Read, world_type: WorldType) -> Result<StatusElement, String> {
		let location_x = stream.read_u8().map_err(|e| format!("Failed to read X location: {}", e))?;
		let location_y = stream.read_u8().map_err(|e| format!("Failed to read Y location: {}", e))?;

		let step_x = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read X step: {}", e))?;
		let step_y = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read Y step: {}", e))?;
		let cycle = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read cycle: {}", e))?;
		let param1 = stream.read_u8().map_err(|e| format!("Failed to read param1: {}", e))?;
		let param2 = stream.read_u8().map_err(|e| format!("Failed to read param2: {}", e))?;
		let param3 = stream.read_u8().map_err(|e| format!("Failed to read param3: {}", e))?;
		let follower = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read follower: {}", e))?;
		let leader = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read leader: {}", e))?;
		let under_element_id = stream.read_u8().map_err(|e| format!("Failed to read under ID: {}", e))?;
		let under_colour = stream.read_u8().map_err(|e| format!("Failed to read under colour: {}", e))?;
		let _internal_code_pointer = stream.read_i32::<LittleEndian>().map_err(|e| format!("Failed to read internal code pointer: {}", e))?;
		let code_current_instruction = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read current code instruction: {}", e))?;
		let code_length = stream.read_i16::<LittleEndian>().map_err(|e| format!("Failed to read code length: {}", e))?;

		match world_type {
			WorldType::Zzt => {
				for _ in 0 .. 8 {
					let _padding_byte = stream.read_u8().map_err(|e| format!("Failed to read padding bytes: {}", e))?;
				}
			}
			_ => {}
		}

		let code_source;
		if code_length < 0 {
			code_source = CodeSource::Bound((-code_length) as usize);
		} else {
			let mut code = DosString::new();
			for _ in 0 .. code_length {
				let c = stream.read_u8().map_err(|e| format!("Failed to read code: {}", e))?;
				code.push(c);
			}
			code_source = CodeSource::Owned(code);
		}

		Ok(StatusElement {
			location_x,
			location_y,
			step_x,
			step_y,
			cycle,
			param1,
			param2,
			param3,
			follower,
			leader,
			under_element_id,
			under_colour,
			code_current_instruction,
			code_source,
		})
	}

	fn write(&self, stream: &mut std::io::Write, world_type: WorldType) -> Result<(), String> {
		stream.write_u8(self.location_x).map_err(|e| format!("Failed to write X location: {}", e))?;
		stream.write_u8(self.location_y).map_err(|e| format!("Failed to write Y location: {}", e))?;
		stream.write_i16::<LittleEndian>(self.step_x).map_err(|e| format!("Failed to write X step: {}", e))?;
		stream.write_i16::<LittleEndian>(self.step_y).map_err(|e| format!("Failed to write Y step: {}", e))?;
		stream.write_i16::<LittleEndian>(self.cycle).map_err(|e| format!("Failed to write cycle: {}", e))?;
		stream.write_u8(self.param1).map_err(|e| format!("Failed to write param1: {}", e))?;
		stream.write_u8(self.param2).map_err(|e| format!("Failed to write param2: {}", e))?;
		stream.write_u8(self.param3).map_err(|e| format!("Failed to write param3: {}", e))?;
		stream.write_i16::<LittleEndian>(self.follower).map_err(|e| format!("Failed to write follower: {}", e))?;
		stream.write_i16::<LittleEndian>(self.leader).map_err(|e| format!("Failed to write leader: {}", e))?;
		stream.write_u8(self.under_element_id).map_err(|e| format!("Failed to write under ID: {}", e))?;
		stream.write_u8(self.under_colour).map_err(|e| format!("Failed to write under colour: {}", e))?;
		stream.write_i32::<LittleEndian>(0).map_err(|e| format!("Failed to write pointer: {}", e))?;
		stream.write_i16::<LittleEndian>(self.code_current_instruction).map_err(|e| format!("Failed to write current code instruction: {}", e))?;
		stream.write_i16::<LittleEndian>(self.code_source.get_save_code_length()).map_err(|e| format!("Failed to write code length: {}", e))?;

		match world_type {
			WorldType::Zzt => {
				for _ in 0 .. 8 {
					stream.write_u8(0).map_err(|e| format!("Failed to write padding bytes: {}", e))?;
				}
			}
			_ => {}
		}

		if let CodeSource::Owned(ref code) = self.code_source {
			for c in &code.data {
				stream.write_u8(*c).map_err(|e| format!("Failed to write code: {}", e))?;
			}
		}

		Ok(())
	}
}

impl Default for StatusElement {
	fn default() -> StatusElement {
		StatusElement {
			location_x: 0,
			location_y: 0,
			step_x: 0,
			step_y: 0,
			cycle: 1,
			param1: 0,
			param2: 0,
			param3: 0,
			follower: -1,
			leader: -1,
			under_element_id: 0,
			under_colour: 0,
			code_current_instruction: 0,
			code_source: CodeSource::Owned(DosString::new()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use std::path::Path;
	use std::io::Cursor;

	#[test] fn basic_save_load() {
		let zzt_file_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/BASIC.ZZT"));
		let mut zzt_file = std::fs::File::open(zzt_file_path).unwrap();

		let world = World::parse(&mut zzt_file).unwrap();

		let mut out_buf = vec![];
		world.write(&mut out_buf);

		let mut out_buf_cursor = Cursor::new(out_buf.as_slice());
		let world_reloaded = World::parse(&mut out_buf_cursor).unwrap();

		assert_eq!(world, world_reloaded);
	}
}
