use crate::direction::*;
use crate::sounds::*;
use zzt_file_format::dosstring::DosString;

/// Board messages will be applied after the current status is finished being processed. They are
/// sent all the way out to the front-end, which are then intercepted and/or passed back into
/// `ZztEngine::process_board_message`.
#[derive(Debug, Clone)]
pub enum BoardMessage {
	/// The board should be switched to the board with the given `new_board_index`, when the player
	/// walked off the side of the board in the given `direction`.
	SwitchBoard {
		new_board_index: usize,
		direction: Direction,
	},
	/// The board should be switched to the board with the given `destination_board_index`, when the
	/// player used a passage with the given `passage_colour`.
	TeleportToBoard {
		destination_board_index: u8,
		passage_colour: u8,
	},
	/// A flashy caption message should be shown that only appears one time and is never shown again
	/// on subsequent requests to show that notification.
	ShowOneTimeNotification(OneTimeNotification),
	/// A scroll should be opened with the given `title` and given `content_lines`. Note that if
	/// `content_lines` has only one entry, a flashy caption should appear instead of opening a
	/// scroll.
	OpenScroll {
		title: DosString,
		content_lines: Vec<DosString>,
	},
	/// Any open scroll should be closed.
	CloseScroll,
	/// Enter was pressed while a scroll was open, on the line given by `line_index`.
	EnterPressedInScroll{line_index: usize},
	/// The sounds in the given array should be played from the system speaker.
	/// *Note* that this is not handled by the ZztEngine, and must be implemented by the front-end
	/// for it to do anything.
	PlaySoundArray(Vec<SoundEntry>, SoundPriority),
	/// If there is any sound playing, this should clear it.
	ClearPlayingSound,
	/// An input for entering a filename to save to should be shown.
	OpenSaveGameInput,
	/// The current state of the game world should be saved to a file with the given name.
	SaveGameToFile(DosString),
	/// The debug command line input should be shown.
	OpenDebugInput,
	/// The given debug command should be applied. (eg. `zap`, `health` etc.).
	DebugCommand(DosString),
	/// A scroll was open and a link was clicked within the scroll with the given destination text.
	LinkClicked(DosString),
	/// The game should be paused.
	PauseGame,
	/// If the title screen is currently shown, then the game should switch to be in-game, on the
	/// correct board, and then pause the game.
	PlayGame,
	/// Should open the scroll with all the .ZZT files.
	OpenWorldSelection,
	/// Should open the scroll with all the .SAV files.
	OpenSaveSelection,
	/// Should load the world with the given name, and load it into the engine.
	OpenWorld{filename: DosString},
	/// The input to end the current game should be shown.
	OpenEndGameConfirmation,
	/// The input to quit RUZZT should be shown.
	OpenQuitConfirmation,
	/// Should return to the title screen.
	ReturnToTitleScreen,
	/// Should stop running altogether.
	Quit,
}

/// Types of "one-time notifications". Each type is displayed once in a caption the first time it is
/// requested and never shown again on subsequent requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OneTimeNotification {
	BlockedByWater,
	NoAmmo,
	NoTorches,
	RoomIsDark,
	LightTorchInLitRoom,
	ShootingNotAllowed,
	ForestCleared,
	PickUpEnergizer,
}

impl OneTimeNotification {
	/// Get the caption text to be shown for the notification.
	pub fn message_string(self) -> DosString {
		match self {
			OneTimeNotification::BlockedByWater => DosString::from_slice(b"Your way is blocked by water."),
			OneTimeNotification::NoAmmo => DosString::from_slice(b"You don't have any ammo!"),
			OneTimeNotification::NoTorches => DosString::from_slice(b"You don't have any torches!"),
			OneTimeNotification::RoomIsDark => DosString::from_slice(b"Room is dark - you need to light a torch!"),
			OneTimeNotification::LightTorchInLitRoom => DosString::from_slice(b"Don't need torch - room is not dark!"),
			OneTimeNotification::ShootingNotAllowed => DosString::from_slice(b"Can't shoot in this place!"),
			OneTimeNotification::ForestCleared => DosString::from_slice(b"A path is cleared through the forest."),
			OneTimeNotification::PickUpEnergizer => DosString::from_slice(b"Energizer - You are invincible"),
		}
	}
}
