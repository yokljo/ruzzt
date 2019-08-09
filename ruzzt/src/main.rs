mod sound;

use sdl2::image::{LoadTexture, INIT_PNG};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::{WindowCanvas, Texture};
use sdl2::audio::AudioSpecDesired;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use num::FromPrimitive;

use ruzzt_engine::board_message::BoardMessage;
use ruzzt_engine::engine::RuzztEngine;
use ruzzt_engine::console::{ConsoleState, SCREEN_HEIGHT, SCREEN_WIDTH};
use zzt_file_format::dosstring::DosString;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn get_ms_from_duration(duration: std::time::Duration) -> usize {
	(duration.as_secs() * 1000) as usize + duration.subsec_millis() as usize
}

fn world_selection_info(world_name: &[u8]) -> &[u8] {
	match world_name {
		b"CAVES" => b"The Caves of ZZT",
		b"CITY" => b"Underground City of ZZT",
		b"DUNGEONS" => b"The Dungeons of ZZT",
		b"TOUR" => b"Guided Tour of ZZT's Other Worlds",
		b"TOWN" => b"The Town of ZZT",
		_ => b"",
	}
}

struct WorldSelectionState {
	entries: Vec<std::path::PathBuf>,
}

enum CustomScrollState {
	None,
	WorldSelection{world_selection_state: WorldSelectionState, play_immediately: bool},
}

struct ZztConsole {
	engine: RuzztEngine,
	current_console_state: ConsoleState,
	current_run_time_ms: usize,
	custom_scroll_state: CustomScrollState,
}

impl ZztConsole {
	fn new() -> ZztConsole {
		let command_arguments = clap::App::new("ruzzt")
			.about("A ZZT clone")
			.arg(clap::Arg::with_name("WORLD_FILE")
				.help("A ZZT world file to load on startup.")
				.required(false)
				.index(1))
			.arg(clap::Arg::with_name("board")
				.short("b")
				.value_name("BOARD")
				.help("Starts on the given board number"))
			.get_matches();

		let mut console = ZztConsole {
			engine: RuzztEngine::new(),
			current_console_state: ConsoleState::new(),
			current_run_time_ms: 0,
			custom_scroll_state: CustomScrollState::None,
		};

		let board_index = if let Some(board_name) = command_arguments.value_of("board") {
			if let Ok(board_index) = board_name.parse() {
				Some(board_index)
			} else {
				eprintln!("Board index must be an integer");
				None
			}
		} else {
			None
		};

		if let Some(init_world_name) = command_arguments.value_of("WORLD_FILE") {
			let mut file = std::fs::File::open(init_world_name).unwrap();
			let world = zzt_file_format::World::parse(&mut file).unwrap();

			console.engine.load_world(world, board_index);

			if board_index.is_some() {
				console.engine.set_in_title_screen(false);
				let mut board_messages = vec![];
				console.engine.board_simulator.on_player_entered_board(&mut board_messages);
			}
		} else {
			console.open_world(&DosString::from_slice(b"TOWN.ZZT"));
		}

		console
	}

	fn draw_screen(&mut self, canvas: &mut WindowCanvas, dosfont_tex: &mut Texture, redraw_all: bool) {
		for y in 0 .. SCREEN_HEIGHT {
			for x in 0 .. SCREEN_WIDTH {
				let ref screen_char = self.engine.console_state.screen_chars[y][x];
				let ref old_screen_char = self.current_console_state.screen_chars[y][x];

				let mut blinking = false;

				let mut back_num = screen_char.background as u8;
				if back_num >= 8 {
					back_num -= 8;
					blinking = true;
				}

				if screen_char != old_screen_char || redraw_all || blinking {
					let back_rgb = ruzzt_engine::console::ConsoleColour::from_u8(back_num).unwrap().to_rgb();

					let fore_rgb = screen_char.foreground.to_rgb();

					let char_rect = Rect::new(8 * (screen_char.char_code as i32), 0, 8, 14);

					let dest_rect = Rect::new(8 * (x as i32), 14 * (y as i32), 8, 14);

					// Draw the character background:
					canvas.set_draw_color(sdl2::pixels::Color::RGB(back_rgb.0, back_rgb.1, back_rgb.2));
					canvas.fill_rect(dest_rect).ok();

					if !blinking || self.current_run_time_ms % 450 < 225 {
						// Draw the character foreground:
						dosfont_tex.set_color_mod(fore_rgb.0, fore_rgb.1, fore_rgb.2);
						canvas.copy(&dosfont_tex, Some(char_rect), Some(dest_rect)).expect("Render failed");
					}

					self.current_console_state.screen_chars[y][x] = *screen_char;
				}
			}
		}
	}

	fn open_world_selection_scroll(&mut self, scroll_title: &[u8], file_extension: &str, play_immediately: bool) {
		let mut files = vec![];
		let mut world_selection_state = WorldSelectionState{entries: vec![]};

		if let Ok(read_dir) = std::fs::read_dir(".") {
			for dir_file in read_dir {
				if let Ok(dir_file_entry) = dir_file {
					if let Ok(mut dir_file_entry_name) = dir_file_entry.file_name().into_string() {
						dir_file_entry_name.make_ascii_uppercase();
						if dir_file_entry_name.ends_with(file_extension) {
							dir_file_entry_name.truncate(dir_file_entry_name.len() - file_extension.len());
							let world_name = DosString::from_str(&dir_file_entry_name);
							let mut scroll_line = world_name.clone();
							while scroll_line.len() < 11 {
								scroll_line += b" ";
							}
							scroll_line += world_selection_info(&world_name.data);
							world_selection_state.entries.push(dir_file_entry.path());
							files.push(scroll_line);
						}
					}
				}
			}
		}
		files.push(DosString::from_slice(b"Exit"));
		self.engine.open_scroll(DosString::from_slice(scroll_title), files);
		self.custom_scroll_state = CustomScrollState::WorldSelection{world_selection_state, play_immediately};
	}

	pub fn open_world(&mut self, filename: &DosString) {
		let filename_str = filename.to_string(false);
		if let Ok(read_dir) = std::fs::read_dir(".") {
			for dir_file in read_dir {
				if let Ok(dir_file_entry) = dir_file {
					if let Ok(mut dir_file_entry_name) = dir_file_entry.file_name().into_string() {
						dir_file_entry_name.make_ascii_uppercase();
						if dir_file_entry_name == filename_str {
							let mut file = std::fs::File::open(dir_file_entry.path()).unwrap();
							let world = zzt_file_format::World::parse(&mut file).unwrap();
							self.engine.load_world(world, None);
							break;
						}
					}
				}
			}
		}
	}

	pub fn run(&mut self) {
		println!("");
		println!("  Corroded version -- Thank you for playing RUZZT.");
		let scale = 2;

		let sdl_context = sdl2::init().unwrap();

		//
		// Init audio.
		//

		let audio_subsystem = sdl_context.audio().unwrap();

		let desired_spec = AudioSpecDesired {
			freq: Some(44100),
			channels: Some(1),
			// Use the default default sample count.
			samples: None,
		};

		let mut audio_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
			sound::SoundPlayer::new(spec)
		}).unwrap();

		// Start audio playback.
		audio_device.resume();

		//
		// Init video.
		//

		let render_width = 640;
		let render_height = 350;

		let sdl_video = sdl_context.video().unwrap();
		let _sdl_image = sdl2::image::init(INIT_PNG).unwrap();
		let window = sdl_video.window("RUZZT", render_width * scale, render_height * scale)
			.position_centered()
			//.fullscreen_desktop()
			.build()
			.unwrap();

		let (window_width, window_height) = window.size();

		let mut canvas = window.into_canvas().software().build().unwrap();
		let texture_creator = canvas.texture_creator();

		let dosfont_file = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/res/dosfont.png"));
		let mut dosfont_tex = texture_creator.load_texture(dosfont_file).unwrap();

		let mut running = true;

		canvas.set_scale(scale as f32, scale as f32).ok();
		canvas.set_viewport(Rect::new(((window_width / scale) as i32 / 2 - render_width as i32 / 2) as i32, ((window_height / scale) as i32 / 2 - render_height as i32 / 2) as i32, render_width, render_height));

		sdl_context.mouse().show_cursor(false);

		let start_time_ms = get_ms_from_duration(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
		let mut last_time_ms = start_time_ms;

		// Rough calculation: 161 cycles happens in about 17.3 seconds:
		// 0.10745341614906832 seconds per cycle.
		// 9.306358381502891 Hz
		// Round that to 9.3 Hz
		let rate_hz: f64 = 9.3;

		self.draw_screen(&mut canvas, &mut dosfont_tex, true);

		while running {
			let in_typing_mode = self.engine.in_typing_mode();
			let mut engine_event = ruzzt_engine::event::Event::None;
			let mut engine_typing_event = ruzzt_engine::event::TypingEvent::None;

			for event in sdl_context.event_pump().unwrap().poll_iter() {
				match event {
					Event::Quit{..} => {
						running = false;
					}
					Event::Window{..} => {
						self.draw_screen(&mut canvas, &mut dosfont_tex, true);
					}
					Event::KeyDown {keycode: keycode_opt, keymod, ..} => {
						if let Some(keycode) = keycode_opt {
							match keycode {
								Keycode::F1 => {
									running = false;
								}
								_ => {}
							}

							let shift_held = keymod.contains(sdl2::keyboard::LSHIFTMOD);

							if in_typing_mode {
								if keycode as i32 >= 0x20 && keycode as i32 <= 0x7e {
									let mut char_code = keycode as u8;
									if shift_held {
										// Very friendly for non-US keyboards:
										if char_code >= b'a' && char_code <= b'z' {
											char_code -= b'a' - b'A';
										} else if char_code >= b'0' && char_code <= b'9' {
											let upper_numbers = b")!@#$%^&*(";
											char_code = upper_numbers[(char_code - b'0') as usize];
										} else {
											let from_chars = b"`-=[]\\;',./";
											let to_chars = b"~_+{}|:\"<>?";
											if let Some((index, _)) = from_chars.iter().enumerate().find(|(_, c)| **c == char_code) {
												char_code = to_chars[index];
											}
										}
									}
									engine_typing_event = ruzzt_engine::event::TypingEvent::Char(char_code);
								} else {
									match keycode {
										Keycode::Escape => {
											engine_typing_event = ruzzt_engine::event::TypingEvent::Escape;
										}
										Keycode::Return => {
											engine_typing_event = ruzzt_engine::event::TypingEvent::Enter;
										}
										Keycode::Left | Keycode::Backspace => {
											engine_typing_event = ruzzt_engine::event::TypingEvent::Backspace;
										}
										_ => {}
									}
								}
							} else {
								match keycode {
									Keycode::Escape => {
										engine_event = ruzzt_engine::event::Event::Escape;
									}
									Keycode::Left => {
										engine_event = if shift_held {
											ruzzt_engine::event::Event::ShootLeft
										} else {
											ruzzt_engine::event::Event::Left
										}
									}
									Keycode::Right => {
										engine_event = if shift_held {
											ruzzt_engine::event::Event::ShootRight
										} else {
											ruzzt_engine::event::Event::Right
										}
									}
									Keycode::Up => {
										engine_event = if shift_held {
											ruzzt_engine::event::Event::ShootUp
										} else {
											ruzzt_engine::event::Event::Up
										}
									}
									Keycode::Down => {
										engine_event = if shift_held {
											ruzzt_engine::event::Event::ShootDown
										} else {
											ruzzt_engine::event::Event::Down
										}
									}
									Keycode::P => {
										if self.engine.in_title_screen {
											engine_event = ruzzt_engine::event::Event::PlayGame;
										} else {
											engine_event = ruzzt_engine::event::Event::PauseGame;
										}
									}
									Keycode::Q => {
										engine_event = ruzzt_engine::event::Event::Quit;
									}
									Keycode::PageUp => {
										engine_event = ruzzt_engine::event::Event::PageUp;
									}
									Keycode::PageDown => {
										engine_event = ruzzt_engine::event::Event::PageDown;
									}
									Keycode::R => {
										engine_event = ruzzt_engine::event::Event::RestoreGame;
									}
									Keycode::Return => {
										engine_event = ruzzt_engine::event::Event::Enter;
									}
									Keycode::Space => {
										engine_event = ruzzt_engine::event::Event::ShootFlow;
									}
									Keycode::S => {
										engine_event = ruzzt_engine::event::Event::SaveGame;
									}
									Keycode::Slash => {
										if shift_held {
											engine_event = ruzzt_engine::event::Event::Debug;
										}
									}
									Keycode::T => {
										engine_event = ruzzt_engine::event::Event::LightTorch;
									}
									Keycode::W => {
										engine_event = ruzzt_engine::event::Event::OpenWorldSelection;
									}
									_ => {}
								}
							}
						}
					}
					_ => {}
				}
			}

			let mut board_messages = if in_typing_mode {
				self.engine.process_typing(engine_typing_event)
			} else {
				let mut board_messages = vec![];
				for _ in 0 ..= if self.engine.should_simulate_fast() { 2 } else { 0 } {
					let global_time_passed_seconds: f64 = self.current_run_time_ms as f64 / 1000.;
					board_messages.extend(self.engine.step(engine_event, global_time_passed_seconds));
					engine_event = ruzzt_engine::event::Event::None;
				}
				self.engine.update_screen();
				board_messages
			};

			let mut new_sounds_list = vec![];
			let mut should_clear_sound = false;

			let applied_board_message = !board_messages.is_empty();

			while !board_messages.is_empty() {
				let processing_board_messages = std::mem::replace(&mut board_messages, vec![]);
				for board_message in processing_board_messages {
					match board_message {
						BoardMessage::PlaySoundArray(ref sound_array, priority) => {
							new_sounds_list.push((sound_array.clone(), priority));
						}
						BoardMessage::ClearPlayingSound => {
							should_clear_sound = true;
						}
						BoardMessage::Quit => {
							running = false;
						}
						BoardMessage::OpenWorldSelection => {
							self.open_world_selection_scroll(b"RUZZT Worlds", ".ZZT", false);
						}
						BoardMessage::OpenSaveSelection => {
							self.open_world_selection_scroll(b"Saved Games", ".SAV", true);
						}
						BoardMessage::EnterPressedInScroll{line_index} => {
							match self.custom_scroll_state {
								CustomScrollState::None => {}
								CustomScrollState::WorldSelection{ref world_selection_state, play_immediately} => {
									if let Some(file_path) = world_selection_state.entries.get(line_index) {
										let mut file = std::fs::File::open(file_path).unwrap();
										let world = zzt_file_format::World::parse(&mut file).unwrap();
										self.engine.load_world(world, None);
										if play_immediately {
											self.engine.set_in_title_screen(false);
										}
									}
								}
							}
							self.custom_scroll_state = CustomScrollState::None;
						}
						BoardMessage::OpenWorld{ref filename} => {
							self.open_world(filename);
						}
						_ => {}
					}
					let extra_board_messages = self.engine.process_board_message(board_message);
					board_messages.extend(extra_board_messages);
				}
			}

			for (new_sounds, priority) in new_sounds_list {
				audio_device.lock().play_sounds(new_sounds, priority);
			}

			if should_clear_sound {
				audio_device.lock().clear_sound_queue();
			}

			self.draw_screen(&mut canvas, &mut dosfont_tex, false);

			canvas.present();

			let current_time_ms = get_ms_from_duration(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());

			if !applied_board_message {
				let max_delay = (1000. / rate_hz) as usize;
				let frame_length_ms = current_time_ms - last_time_ms;
				if frame_length_ms < max_delay {
					let delay = max_delay - frame_length_ms;
					let delay_duration = std::time::Duration::from_millis(if self.engine.should_simulate_fast() { 10 } else { delay as u64 });
					//let delay_duration = std::time::Duration::from_millis(0);
					// TODO: This could intelligently wait for the minimum of the time to the next
					// simulation step, and the time till the next screen blink.
					std::thread::sleep(delay_duration);
				}
			}

			let time_since_start_ms = current_time_ms - start_time_ms;
			self.current_run_time_ms = time_since_start_ms;

			last_time_ms = get_ms_from_duration(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
		}
	}
}

pub fn main() {
	color_backtrace::install();

	let mut console = ZztConsole::new();
	console.run();
}
