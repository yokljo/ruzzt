/// Represents a game controller input event.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Event {
	/// A no-op event.
	None,
	/// Move west was pressed.
	Left,
	/// Move east was pressed.
	Right,
	/// Move north was pressed.
	Up,
	/// Move south was pressed.
	Down,
	/// The page-up key was pressed (used when scrolls are open).
	PageUp,
	/// The page-down key was pressed (used when scrolls are open).
	PageDown,
	/// The enter key was pressed (used in scrolls to either click a selected link, or close the
	/// scroll).
	Enter,
	/// The escape key was pressed (quit the game, or close a scroll, etc.).
	Escape,
	/// Shoot in the direction the player is already moving (its step X/Y determine this).
	ShootFlow,
	/// Shoot west was pressed.
	ShootLeft,
	/// Shoot east was pressed.
	ShootRight,
	/// Shoot north was pressed.
	ShootUp,
	/// Shoot south was pressed.
	ShootDown,
	/// The key to light a torch was pressed (usually T).
	LightTorch,
	/// The key to pause the game was pressed (usually P, only applies in-game).
	PauseGame,
	/// The key to open the save game input box was pressed (usually S).
	SaveGame,
	/// The key to open the debug command input box was pressed (usually ?).
	Debug,
	/// The key to open the world selection scroll was pressed (usually W, only applies in the title
	/// screen).
	OpenWorldSelection,
	/// The key to start playing the game was pressed (usually P, only applies in the title screen).
	PlayGame,
	/// The key to open the saved-game selection scroll was pressed (usually R, only applies in the
	/// title screen).
	RestoreGame,
	/// The key to quit the game was pressed (usually Q).
	/// Note that this is different from Escape in very particular circumstances.
	Quit,
	/// The key to open the "About" scroll was pressed (usually A, only applies in the title
	/// screen).
	OpenAbout,
	/// The key to open the highscores scroll was pressed (usually H, only applies in the title
	/// screen).
	OpenHighScores,
	/// The key to open the world editor was pressed (usually E, only applies in the title
	/// screen).
	OpenEditor,
	/// The key to use the game speed selector was pressed (usually S, only applies in the title
	/// screen).
	ChangeGameSpeed,
}

/// Represents a text input event.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypingEvent {
	/// A no-op event.
	None,
	/// The key with the given ASCII code was pressed.
	Char(u8),
	/// The Backspace key was pressed.
	Backspace,
	/// The Enter key was pressed.
	Enter,
	/// The Escape key was pressed.
	Escape,
}
