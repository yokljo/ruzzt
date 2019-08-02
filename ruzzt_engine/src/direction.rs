/// A cardinal direction to move in (or Idle).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Direction {
	West,
	East,
	North,
	South,
	Idle,
}

impl Direction {
	/// Get the x/y unit offset for the direction.
	pub fn to_offset(self) -> (i16, i16) {
		match self {
			Direction::West => (-1, 0),
			Direction::East => (1, 0),
			Direction::North => (0, -1),
			Direction::South => (0, 1),
			Direction::Idle => (0, 0),
		}
	}
	
	/// Get a direction associated with an x/y unit offset, or Idle for anything else.
	pub fn from_offset(x: i16, y: i16) -> Direction {
		match (x, y) {
			(-1, 0) => Direction::West,
			(1, 0) => Direction::East,
			(0, -1) => Direction::North,
			(0, 1) => Direction::South,
			_ => Direction::Idle,
		}
	}
	
	/// Return the opposite direction, ie. East<->West, North<->South, Idle<->Idle.
	pub fn opposite(self) -> Direction {
		match self {
			Direction::West => Direction::East,
			Direction::East => Direction::West,
			Direction::North => Direction::South,
			Direction::South => Direction::North,
			Direction::Idle => Direction::Idle,
		}
	}
	
	/// Return the direction 90 degrees clockwise.
	pub fn cw(self) -> Direction {
		match self {
			Direction::North => Direction::East,
			Direction::East => Direction::South,
			Direction::South => Direction::West,
			Direction::West => Direction::North,
			Direction::Idle => Direction::Idle,
		}
	}
	
	/// Return the direction 90 degrees counter-clockwise.
	pub fn ccw(self) -> Direction {
		match self {
			Direction::North => Direction::West,
			Direction::East => Direction::North,
			Direction::South => Direction::East,
			Direction::West => Direction::South,
			Direction::Idle => Direction::Idle,
		}
	}
}
