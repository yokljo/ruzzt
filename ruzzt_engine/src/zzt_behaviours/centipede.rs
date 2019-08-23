use crate::behaviour::*;
use crate::board_simulator::*;
use crate::event::*;
use crate::direction::*;

use rand::Rng;

use zzt_file_format::*;

use crate::zzt_behaviours::monster_interactions::*;

struct HeadStepContext<'l> {
	new_step_x: i16,
	new_step_y: i16,
	actions: Vec<Action>,
	continuation: Option<Box<dyn ActionContinuation>>,
	status_index_for_head: usize,
	status: &'l StatusElement,
	status_index: usize,
	sim: &'l BoardSimulator,
}

impl<'l> HeadStepContext<'l> {
	// The player doesn't count as blocked, so a centipede will happily walk into it.
	fn is_blocked_and_not_player(&self, x: i16, y: i16) -> bool {
		let (player_x, player_y) = self.sim.get_player_location();
		if x == player_x && y == player_y {
			false
		} else {
			let dest_behaviour = self.sim.behaviour_for_pos(x, y);
			dest_behaviour.blocked(false) == BlockedStatus::Blocked
		}
	}

	fn reverse_centipede(&mut self) {
		if self.status.follower < 0 {
			// This is a head with no followers, so just turn around 180 degrees.
			let opp_is_blocked = self.is_blocked_and_not_player(self.status.location_x as i16 - self.new_step_x, self.status.location_y as i16 - self.new_step_y);
			if !opp_is_blocked {
				self.new_step_x = -self.new_step_x;
				self.new_step_y = -self.new_step_y;
			} else {
				// Do nothing
			}
		} else {
			// Change direction by 180 degrees, and move head to opposite end of
			// centipede, AFTER the last tail segment. If that position is blocked,
			// don't do anything.

			// In the original game, when a worm turns around, all you see is the
			// step after it moves, after it has already turned around. You can tell
			// that the worm is turned around by swapping the types of of the
			// head and end of tail segments because if you save the game just as it
			// is turning around, the save file has the head at the other end. This
			// means the original game simply doesn't re-render the worm when it
			// turns around, creating a lag effect. We will not simulate that.

			self.actions.push(Action::SetTileElementIdAndColour {
				x: self.status.location_x as i16,
				y: self.status.location_y as i16,
				element_id: Some(ElementType::Segment as u8),
				colour: None,
			});

			let mut current_status_index: usize = self.status_index;

			// The position of the segment just before the end of the tail.
			let mut before_end_of_tail_x = self.status.location_x as i16 + self.new_step_x;
			let mut before_end_of_tail_y = self.status.location_y as i16 + self.new_step_y;
			loop {
				let current_status = &self.sim.status_elements[current_status_index];

				self.actions.push(Action::SetLeader{
					status_index: current_status_index,
					leader: current_status.follower,
				});
				self.actions.push(Action::SetFollower{
					status_index: current_status_index,
					follower: current_status.leader,
				});

				if current_status.follower >= 0 {
					self.actions.push(Action::SetStep{
						status_index: current_status_index,
						step_x: -current_status.step_x,
						step_y: -current_status.step_y,
					});

					before_end_of_tail_x = current_status.location_x as i16;
					before_end_of_tail_y = current_status.location_y as i16;
					current_status_index = current_status.follower as usize;
				} else {
					// This will be the new head.
					self.actions.push(Action::SetTileElementIdAndColour {
						x: current_status.location_x as i16,
						y: current_status.location_y as i16,
						element_id: Some(ElementType::Head as u8),
						colour: None,
					});

					self.status_index_for_head = current_status_index;

					// When the centipede just turns around and is about to hit a wall,
					// for some reason it always goes down for vertical walls or left for
					// horizontal walls.

					self.new_step_x = current_status.location_x as i16 - before_end_of_tail_x;
					self.new_step_y = current_status.location_y as i16 - before_end_of_tail_y;
					let next_pos_is_blocked = self.is_blocked_and_not_player(current_status.location_x as i16 + self.new_step_x, current_status.location_y as i16 + self.new_step_y);
					if next_pos_is_blocked {
						std::mem::swap(&mut self.new_step_x, &mut self.new_step_y);
						// Down for vertical walls
						if self.new_step_x == 0 { self.new_step_y = self.new_step_y.abs(); }
						// Left for horizontal walls.
						if self.new_step_y == 0 { self.new_step_x = -self.new_step_x.abs(); }

						let dest_pos_is_blocked = self.is_blocked_and_not_player(current_status.location_x as i16 + self.new_step_x, current_status.location_y as i16 + self.new_step_y);
						if dest_pos_is_blocked {
							self.new_step_x = -self.new_step_x;
							self.new_step_y = -self.new_step_y;
						}
					}

					break;
				}
			}
		}
	}

	// Randomly change direction sometimes according to AI settings.
	fn do_intelligence_and_deviance(&mut self) {
		// NOTE: This logic was derived from the ZZT.EXE disassembly.
		let mut rng = rand::thread_rng();

		let (player_x, player_y) = self.sim.get_player_location();

		let mut changed_direction = false;
		// Check aligned on the X axis.
		if self.status.location_x as i16 == player_x {
			let random_int: u8 = rng.gen_range(0, 10);
			if self.status.param1 > random_int {
				self.new_step_x = 0;
				self.new_step_y = (player_y - self.status.location_y as i16).signum();
				changed_direction = true;
			}
		}

		if !changed_direction {
			// Check aligned on the Y axis.
			if self.status.location_y as i16 == player_y {
				let random_int: u8 = rng.gen_range(0, 10);
				if self.status.param1 > random_int {
					self.new_step_x = (player_x - self.status.location_x as i16).signum();
					self.new_step_y = 0;
					changed_direction = true;
				}
			}
		}

		if !changed_direction {
			// Check deviance.
			let random_int: u8 = rng.gen_range(0, 10) * 4;
			if self.status.param2 > random_int {
				let (rand_step_x, rand_step_y) = self.sim.get_random_step();
				self.new_step_x = rand_step_x;
				self.new_step_y = rand_step_y;
			}
		}
	}

	fn do_step(&mut self) {
		// When there is a segment by itself, it sets the leader to -2 for some reason, then becomes
		// a head.

		let mut rng = rand::thread_rng();

		if self.new_step_x == 0 && self.new_step_y == 0 {
			// If a head has (0, 0) step, it sets the step to the direction of a random non-blocked path.
			// If there are no non-blocked paths: if there are no followers, just sit there. If there
			// are followers, swap head and end of tail and change direction by 180 degrees.
			let n_behaviour = self.sim.behaviour_for_pos(self.status.location_x as i16, self.status.location_y as i16 - 1);
			let s_behaviour = self.sim.behaviour_for_pos(self.status.location_x as i16, self.status.location_y as i16 + 1);
			let e_behaviour = self.sim.behaviour_for_pos(self.status.location_x as i16 + 1, self.status.location_y as i16);
			let w_behaviour = self.sim.behaviour_for_pos(self.status.location_x as i16 - 1, self.status.location_y as i16);

			let mut free_dirs = vec![];
			if n_behaviour.blocked(false) == BlockedStatus::NotBlocked { free_dirs.push(Direction::North); }
			if s_behaviour.blocked(false) == BlockedStatus::NotBlocked { free_dirs.push(Direction::South); }
			if e_behaviour.blocked(false) == BlockedStatus::NotBlocked { free_dirs.push(Direction::East); }
			if w_behaviour.blocked(false) == BlockedStatus::NotBlocked { free_dirs.push(Direction::West); }

			if free_dirs.is_empty() {
				// If there are no free directions, do nothing.
			} else {
				// This does not take intelligence into account.
				let random_int: usize = rng.gen_range(0, free_dirs.len());

				let new_step = free_dirs[random_int].to_offset();
				self.new_step_x = new_step.0;
				self.new_step_y = new_step.1;
			}
		} else {
			self.do_intelligence_and_deviance();

			let cw_step_x = -self.new_step_y;
			let cw_step_y = self.new_step_x;
			let ccw_step_x = self.new_step_y;
			let ccw_step_y = -self.new_step_x;

			if self.is_blocked_and_not_player(self.status.location_x as i16 + self.new_step_x, self.status.location_y as i16 + self.new_step_y) {
				let cw_is_blocked = self.is_blocked_and_not_player(self.status.location_x as i16 + cw_step_x, self.status.location_y as i16 + cw_step_y);
				let ccw_is_blocked = self.is_blocked_and_not_player(self.status.location_x as i16 + ccw_step_x, self.status.location_y as i16 + ccw_step_y);
				match (cw_is_blocked, ccw_is_blocked) {
					(true, true) => {
						self.reverse_centipede();
					}
					(true, false) => {
						self.new_step_x = ccw_step_x;
						self.new_step_y = ccw_step_y;
					}
					(false, true) => {
						self.new_step_x = cw_step_x;
						self.new_step_y = cw_step_y;
					}
					(false, false) => {
						let mut rng = rand::thread_rng();
						let random_bool: bool = rng.gen();
						if random_bool {
							self.new_step_x = cw_step_x;
							self.new_step_y = cw_step_y;
						} else {
							self.new_step_x = ccw_step_x;
							self.new_step_y = ccw_step_y;
						}
					}
				}
			}
		}

		let mut head_died = false;

		if self.new_step_x != 0 || self.new_step_y != 0 {
			let dest_x = self.status.location_x as i16 + self.new_step_x;
			let dest_y = self.status.location_y as i16 + self.new_step_y;
			if self.sim.has_player_at_location(dest_x, dest_y) {
				head_died = true;
				add_monster_touch_player_actions(self.status.location_x as i16, self.status.location_y as i16, &mut self.actions, self.sim);
				if self.status.follower >= 0 {
					let follower_status = &self.sim.status_elements[self.status.follower as usize];
					self.actions.push(Action::SetTileElementIdAndColour {
						x: follower_status.location_x as i16,
						y: follower_status.location_y as i16,
						element_id: Some(ElementType::Head as u8),
						colour: None,
					});
					self.actions.push(Action::ReprocessSameStatusIndexOnRemoval);
				}
			} else {
				// Find the last segment of the tail, and check if there are any adjacent segment to it
				// that should be joined onto the chain. A segment will not do this if its leader is
				// >= 0, which means it is part of a snake already.
				let mut end_of_tail_index: usize = self.status_index;
				// The position of the segment just before the end of the tail.
				let mut before_end_of_tail_x = self.status.location_x as i16 + self.new_step_x;
				let mut before_end_of_tail_y = self.status.location_y as i16 + self.new_step_y;
				loop {
					let current_status = &self.sim.status_elements[end_of_tail_index];
					if current_status.follower >= 0 {
						before_end_of_tail_x = current_status.location_x as i16;
						before_end_of_tail_y = current_status.location_y as i16;
						end_of_tail_index = current_status.follower as usize;
					} else {
						break;
					}
				}

				// Starting in the opposite direction to the step, go one segment at a time, until no more segments are
				// found in that direction. At this point, swap the x and y parts of the tracing direction vector, then
				// start looking for segments in that direction. Join all these found segments together.

				// This is necessary because the `search_status.leader < 0` check is not sufficient due
				// to the leader values being modified while the algorithm is running.
				let mut joined_status_indices = vec![];

				loop {
					let end_of_tail_status = &self.sim.status_elements[end_of_tail_index];

					let mut segment_joined = false;

					let mut search_offset_x = end_of_tail_status.location_x as i16 - before_end_of_tail_x;
					let mut search_offset_y = end_of_tail_status.location_y as i16 - before_end_of_tail_y;

					let mut try_joining_direction = |search_offset_x, search_offset_y| {
						let search_x = end_of_tail_status.location_x as i16 + search_offset_x;
						let search_y = end_of_tail_status.location_y as i16 + search_offset_y;

						let mut segment_joined_in_current_dir = false;
						if let Some((search_status_index, search_status)) = self.sim.get_first_status_for_pos(search_x, search_y) {
							if let Some(search_tile) = self.sim.get_tile(search_x, search_y) {
								if search_tile.element_id == ElementType::Segment as u8
										&& search_status.leader < 0
										&& !joined_status_indices.contains(&search_status_index) {
									segment_joined_in_current_dir = true;
									joined_status_indices.push(search_status_index);

									// this segment should be joined to the centipede
									self.actions.push(Action::SetLeader{
										status_index: search_status_index,
										leader: end_of_tail_index as i16,
									});
									self.actions.push(Action::SetFollower{
										status_index: end_of_tail_index,
										follower: search_status_index as i16,
									});

									end_of_tail_index = search_status_index;
									before_end_of_tail_x = end_of_tail_status.location_x as i16;
									before_end_of_tail_y = end_of_tail_status.location_y as i16;
								}
							}
						}
						segment_joined_in_current_dir
					};

					if try_joining_direction(search_offset_x, search_offset_y) {
						segment_joined = true;
					} else {
						std::mem::swap(&mut search_offset_x, &mut search_offset_y);

						if try_joining_direction(search_offset_x, search_offset_y) {
							segment_joined = true;
						} else {
							search_offset_x = -search_offset_x;
							search_offset_y = -search_offset_y;

							if try_joining_direction(search_offset_x, search_offset_y) {
								segment_joined = true;
							}
						}
					}

					if !segment_joined { break; }
				}

				self.continuation = Some(Box::new(CentipedeMovementContinuation));
			}
		}

		if !head_died {
			self.actions.push(Action::SetStep{
				status_index: self.status_index_for_head,
				step_x: self.new_step_x,
				step_y: self.new_step_y,
			});
		}
	}
}

/*
When segment simulates and its leader is -2, become a head.

When head simulates, find the last status in the chain by tracing the follower indices as far as it
goes.
Then trace a path from the end to one direction in the following list, then if there is no segment
without a leader that way, go to the next direction and try that

(assume all cycles are set to zero)
   O
@OOO
   O
In this situation, the bottom one joins, then the top one.

   O
@OOOO
   O
In this situation, the whole middle line moves left by one, then the bottom segment joins the group,
then the top one becomes a head.

 O
O@O
 O
In this situation, all segments become heads.

@O
In this situation, the seg joins to the head.

(# is a wall)
 #
#@O
 #
In this situation they both become heads, go figure.

 #
O@O
 #
All become heads.

 #
O@OO
 #
All become heads.

 #
O@OO
Head moves down with left seg, then the right segs join on making a full centipede.



If a head has 0, 0 step:
- don't connect up anything
- it sets the step to the direction of a random non-blocked path, and its
turn is (not necessarily) over. If there are no non-blocked paths, just sit there.

otherwise:
Starting in the opposite direction to the step, go one segment at a time, until no more segments are
found in that direction. At this point, swap the x and y parts of the tracing direction vector, then
start looking for segments in that direction. Join all these found segments together.

If a head with a follower gets stuck and can't turn, then the head becomes the end of the tail and
the end of the tail becomes the head.

If a head has no followers, it wont die even if it's trapped.

If a segment steps when its leader index is -2, it becomes a head.
*/

// Centipede movement needs an action continuation because the HeadBehaviour links up the centipede
// chain then immediately moves.
#[derive(Debug, Clone)]
struct CentipedeMovementContinuation;

impl ActionContinuation for CentipedeMovementContinuation {
	fn next_step(&mut self, _apply_action_report: ApplyActionResultReport, _status_index: usize, status: &StatusElement, sim: &BoardSimulator) -> ActionContinuationResult {
		let mut actions = vec![];

		actions.push(Action::MoveTile {
			from_x: status.location_x as i16,
			from_y: status.location_y as i16,
			to_x: status.location_x as i16 + status.step_x,
			to_y: status.location_y as i16 + status.step_y,
			offset_x: status.step_x,
			offset_y: status.step_y,
			check_push: true,
			is_player: false,
		});

		let mut current_tail_index = status.follower;
		let mut prev_x = status.location_x;
		let mut prev_y = status.location_y;
		while current_tail_index >= 0 {
			let tail_status = &sim.status_elements[current_tail_index as usize];

			let offset_x = prev_x as i16 - tail_status.location_x as i16;
			let offset_y = prev_y as i16 - tail_status.location_y as i16;

			actions.push(Action::SetStep {
				status_index: current_tail_index as usize,
				step_x: offset_x,
				step_y: offset_y,
			});

			actions.push(Action::MoveTile {
				from_x: tail_status.location_x as i16,
				from_y: tail_status.location_y as i16,
				to_x: prev_x as i16,
				to_y: prev_y as i16,
				offset_x,
				offset_y,
				check_push: true,
				is_player: false,
			});
			prev_x = tail_status.location_x;
			prev_y = tail_status.location_y;
			current_tail_index = tail_status.follower;
		}

		ActionContinuationResult {
			actions,
			finished: true,
		}
	}
}

/*
param1 is the intelligence (0 = always randomsize when deviating, 8 = always turn towards player when deviating)
param2 is the "deviance" value (0 = don't randomly choose to turn at all, 8 = frequently turn)
*/
#[derive(Debug, Clone)]
pub struct HeadBehaviour;

impl Behaviour for HeadBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		// This is the index of the head's status, which might change when the centipede turns
		// around.
		let mut step_context = HeadStepContext {
			new_step_x: status.step_x,
			new_step_y: status.step_y,
			actions: vec![],
			continuation: None,
			status_index_for_head: status_index,
			status,
			status_index,
			sim,
		};

		step_context.do_step();

		ActionResult {
			actions: step_context.actions,
			continuation: step_context.continuation,
		}
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		monster_damage(self, x, y, damage_type, sim, actions)
	}

	fn destructable(&self) -> bool {
		true
	}

	fn can_be_squashed(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct SegmentBehaviour;

impl Behaviour for SegmentBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, _sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];
		if status.leader == -1 {
			// The original game sets the leader of a segment to -2 when the leader is -1, which is
			// clearly so it can stay a segment for a whole cycle, to give heads that are processed
			// after this segment an opportunity to join this segment on as their tail.
			actions.push(Action::SetLeader{status_index, leader: -2});
		} else if status.leader == -2 {
			// It's important to use SetTileElementId and not SetTile here because SetTileElementId
			// won't change the order of status elements, which changes the behaviour.
			actions.push(Action::SetTileElementIdAndColour {
				x: status.location_x as i16,
				y: status.location_y as i16,
				element_id: Some(ElementType::Head as u8),
				colour: None,
			});
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		monster_damage(self, x, y, damage_type, sim, actions)
	}

	fn destructable(&self) -> bool {
		true
	}

	fn can_be_squashed(&self) -> bool {
		true
	}
}
