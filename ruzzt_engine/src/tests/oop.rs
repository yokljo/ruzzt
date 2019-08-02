use crate::tests::world_tester::*;

#[test]
fn set_flag() {
	let mut world = TestWorld::new_with_player(1, 1);
	
	let mut tile_set = TileSet::new();
	tile_set.add_object('O', "#set a\n");
	world.insert_tile_and_status(tile_set.get('O'), 10, 10);
	
	assert_eq!(world.world_header().last_matching_flag(DosString::from_str("a")), None);
	world.simulate(1);
	assert_eq!(world.world_header().last_matching_flag(DosString::from_str("a")), Some(0));
}

#[test]
fn move_directions() {
	let mut base_world = TestWorld::new_with_player(1, 1);
	
	let mut tile_set = TileSet::new();
	tile_set.add_object('O', "/n/n/e/s/w/i\n");
	
	let mut world = base_world.clone();
	world.insert_tile_and_status(tile_set.get('O'), 10, 10);
	
	let mut world_1 = base_world.clone();
	world_1.insert_tile_and_status(tile_set.get('O'), 10, 9);
	world_1.status_at(10, 9).code_current_instruction = 2;
	
	let mut world_2 = base_world.clone();
	world_2.insert_tile_and_status(tile_set.get('O'), 10, 8);
	world_2.status_at(10, 8).code_current_instruction = 4;
	
	let mut world_3 = base_world.clone();
	world_3.insert_tile_and_status(tile_set.get('O'), 11, 8);
	world_3.status_at(11, 8).code_current_instruction = 6;
	
	let mut world_4 = base_world.clone();
	world_4.insert_tile_and_status(tile_set.get('O'), 11, 9);
	world_4.status_at(11, 9).code_current_instruction = 8;
	
	let mut world_5 = base_world.clone();
	world_5.insert_tile_and_status(tile_set.get('O'), 10, 9);
	world_5.status_at(10, 9).code_current_instruction = 10;
	
	let mut world_6 = base_world.clone();
	world_6.insert_tile_and_status(tile_set.get('O'), 10, 9);
	world_6.status_at(10, 9).code_current_instruction = 12;
	
	world.simulate(1);
	assert!(world.current_board_equals(world_1));
	
	world.simulate(1);
	assert!(world.current_board_equals(world_2));
	
	world.simulate(1);
	assert!(world.current_board_equals(world_3));
	
	world.simulate(1);
	assert!(world.current_board_equals(world_4));
	
	world.simulate(1);
	assert!(world.current_board_equals(world_5));
	
	world.simulate(1);
	assert!(world.current_board_equals(world_6));
}

/// For some reason, `#go i` doesn't actually progress after it idles, so it is effectively `#end`.
#[test]
fn go_i_doesnt_progress() {
	
	let mut tile_set = TileSet::new();
	tile_set.add_object('O', "#go i\nB\n");
	
	let mut world = TestWorld::new_with_player(1, 1);
	world.insert_tile_and_status(tile_set.get('O'), 10, 10);
	
	let mut original_world = world.clone();
	
	world.simulate(1);
	assert!(world.current_board_equals(original_world));
}

// "A\n/i\nB\n/s\nC\n?i\nD\n?s\nE\n#set a\n/i\nF\n#send g\n:g\nG\n/i\nH\n#go i\nI\n/i\nJ\n#go s\nK\n/i\nL\n#try i\nM\n/i\nN\n#try s\nO\n/i\n"
