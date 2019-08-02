use num_derive::FromPrimitive;

pub const SCREEN_WIDTH: usize = 80;
pub const SCREEN_HEIGHT: usize = 25;

/// A single character in the `ConsoleState`'s buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConsoleChar {
	pub char_code: u8,
	/// Note that background colours 0x8-0xf are actually the same as 0x0-0x7, except they blink on
	/// and off regularly.
	pub background: ConsoleColour,
	pub foreground: ConsoleColour,
}

impl ConsoleChar {
	pub fn new(char_code: u8, background: ConsoleColour, foreground: ConsoleColour) -> ConsoleChar {
		ConsoleChar {
			char_code,
			background,
			foreground,
		}
	}
	
	/// Make an empty ConsoleChar with black foreground and background.
	pub fn black() -> ConsoleChar {
		ConsoleChar {
			char_code: 0,
			background: ConsoleColour::Black,
			foreground: ConsoleColour::Black,
		}
	}
}

/// The current state of the characters displayed in the console.
#[derive(Clone)]
pub struct ConsoleState {
	pub screen_chars: [[ConsoleChar; SCREEN_WIDTH]; SCREEN_HEIGHT],
}

impl ConsoleState {
	/// Create a new ConsoleState with a completely black buffer.
	pub fn new() -> ConsoleState {
		ConsoleState {
			screen_chars: [[ConsoleChar::black(); SCREEN_WIDTH]; SCREEN_HEIGHT],
		}
	}
	
	/// Get the character on the screen at the `x`x`y` position.
	pub fn get_char(&self, x: usize, y: usize) -> ConsoleChar {
		self.screen_chars[y][x]
	}
	
	/// Get the character on the screen at the `x`x`y` position as &mut so it can be modified
	/// directly in place.
	pub fn get_char_mut(&mut self, x: usize, y: usize) -> &mut ConsoleChar {
		&mut self.screen_chars[y][x]
	}
	
	/// Starting at `x`x`y` and moving to the right, place characters of `text` in the console, with
	/// the given `background`/`foreground` colours for all the characters.
	pub fn draw_text_at(&mut self, x: usize, y: usize, text: &[u8], background: ConsoleColour, foreground: ConsoleColour) {
		for (i, char_code) in text.iter().enumerate() {
			*self.get_char_mut(x + i, y) = ConsoleChar::new(*char_code, background, foreground);
		}
	}
}

/// The possible colours that can be displayed in the console.
#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(FromPrimitive)]
#[repr(u8)]
pub enum ConsoleColour {
	Black        = 0x0,
	Blue         = 0x1,
	Green        = 0x2,
	Cyan         = 0x3,
	Red          = 0x4,
	Magenta      = 0x5,
	Brown        = 0x6,
	LightGray    = 0x7,
	DarkGray     = 0x8,
	LightBlue    = 0x9,
	LightGreen   = 0xA,
	LightCyan    = 0xB,
	LightRed     = 0xC,
	LightMagenta = 0xD,
	Yellow       = 0xE,
	White        = 0xF,
}

impl ConsoleColour {
	/// Get the (red, green, blue) values for the console colour.
	pub fn to_rgb(self) -> (u8, u8, u8) {
		match self {
			ConsoleColour::Black        => (0x00, 0x00, 0x00),
			ConsoleColour::Blue         => (0x00, 0x00, 0xAA),
			ConsoleColour::Green        => (0x00, 0xAA, 0x00),
			ConsoleColour::Cyan         => (0x00, 0xAA, 0xAA),
			ConsoleColour::Red          => (0xAA, 0x00, 0x00),
			ConsoleColour::Magenta      => (0xAA, 0x00, 0xAA),
			ConsoleColour::Brown        => (0xAA, 0x55, 0x00),
			ConsoleColour::LightGray    => (0xAA, 0xAA, 0xAA),
			ConsoleColour::DarkGray     => (0x55, 0x55, 0x55),
			ConsoleColour::LightBlue    => (0x55, 0x55, 0xFF),
			ConsoleColour::LightGreen   => (0x55, 0xFF, 0x55),
			ConsoleColour::LightCyan    => (0x55, 0xFF, 0xFF),
			ConsoleColour::LightRed     => (0xFF, 0x55, 0x55),
			ConsoleColour::LightMagenta => (0xFF, 0x55, 0xFF),
			ConsoleColour::Yellow       => (0xFF, 0xFF, 0x55),
			ConsoleColour::White        => (0xFF, 0xFF, 0xFF),
		}
	}
}
