use crate::tests::world_tester::*;

#[test]
fn player_move() {
	let mut world = TestWorld::new();
	
	let mut expected = world.clone();
	world.add_player(20, 20);
	expected.add_player(21, 20);
	
	world.event = Event::Right;
	// Expected step is directly related to move event
	expected.status_at(21, 20).step_x = 1;
	
	world.simulate(1);
	assert!(world.current_board_equals(expected));
}

#[test]
fn push_blocks() {
	let mut world = TestWorld::new_with_player(1, 1);
	
	let mut tile_set = TileSet::new();
	tile_set.add('>', BoardTile::new(ElementType::Pusher, 0xff), Some(StatusElement {
		cycle: 3,
		step_x: 1,
		.. StatusElement::default()
	}));
	tile_set.add('#', BoardTile::new(ElementType::Boulder, 0xff), None);
	let template = TileTemplate::from_text(&tile_set, "
		>########
	");
	
	let mut expected = world.clone();
	let mut expected2 = world.clone();
	
	world.insert_template(&template, 10, 10);
	expected.insert_template(&template, 12, 10);
	expected2.insert_template(&template, 14, 10);

	world.simulate(6);
	assert!(world.current_board_equals(expected));
	world.simulate(6);
	assert!(world.current_board_equals(expected2));
}

#[test]
fn centipede_form_heads() {
	let mut world = TestWorld::new_with_player(1, 1);
	
	let mut tile_set = TileSet::new();
	tile_set.add('O', BoardTile::new(ElementType::Segment, 0xff), Some(StatusElement {
		cycle: 1,
		.. StatusElement::default()
	}));
	tile_set.add('@', BoardTile::new(ElementType::Head, 0xff), Some(StatusElement {
		cycle: 1,
		.. StatusElement::default()
	}));
	tile_set.add('#', BoardTile::new(ElementType::Normal, 0xff), None);
	
	let room_tmpl = TileTemplate::from_text(&tile_set, "
		######
		#.##.#
		#....#
		#.####
		###...
	");
	let worm_tmpl = TileTemplate::from_text(&tile_set, "
		......
		....@.
		.OOOO.
	");
	// These all become heads because the head is "stuck" and doesn't do anything/link up to
	// anything.
	let worm_step1_tmpl = TileTemplate::from_text(&tile_set, "
		......
		....@.
		.@@@@.
	");
	
	world.insert_template(&room_tmpl, 10, 10);
	
	let mut expected_step1 = world.clone();
	let mut expected_step2 = world.clone();
	
	world.insert_template(&worm_tmpl, 10, 10);
	expected_step1.insert_template(&worm_tmpl, 10, 10);
	expected_step2.insert_template(&worm_step1_tmpl, 10, 10);

	world.simulate(1);
	assert!(world.current_board_tiles_equals(expected_step1));
	world.simulate(1);
	assert!(world.current_board_tiles_equals(expected_step2));
}

#[test]
fn centipede_walk() {
	// Test 10 times to hopefully catch possibility of randomness.
	for _ in 0 .. 10 {
		let mut world = TestWorld::new_with_player(1, 1);
		
		let mut tile_set = TileSet::new();
		tile_set.add('O', BoardTile::new(ElementType::Segment, 0xff), Some(StatusElement {
			cycle: 1,
			.. StatusElement::default()
		}));
		tile_set.add('@', BoardTile::new(ElementType::Head, 0xff), Some(StatusElement {
			cycle: 1,
			.. StatusElement::default()
		}));
		tile_set.add('#', BoardTile::new(ElementType::Normal, 0xff), None);
		
		let room_tmpl = TileTemplate::from_text(&tile_set, "
			######
			#.##.#
			#....#
			#.####
			###...
		");
		let worm_tmpl = TileTemplate::from_text(&tile_set, "
			......
			.O....
			.OOO@.
		");
		let worm_step1_tmpl = TileTemplate::from_text(&tile_set, "
			......
			....@.
			.OOOO.
		");
		let worm_step2_tmpl = TileTemplate::from_text(&tile_set, "
			......
			....O.
			.@OOO.
			......
		");
		// It always goes down when switching directions and facing a vertical wall.
		let worm_step3_tmpl = TileTemplate::from_text(&tile_set, "
			......
			......
			.OOOO.
			.@....
		");
		
		world.insert_template(&room_tmpl, 10, 10);
		
		let mut expected_step1 = world.clone();
		let mut expected_step2 = world.clone();
		let mut expected_step3 = world.clone();
		
		world.insert_template(&worm_tmpl, 10, 10);
		expected_step1.insert_template(&worm_step1_tmpl, 10, 10);
		expected_step2.insert_template(&worm_step2_tmpl, 10, 10);
		expected_step3.insert_template(&worm_step3_tmpl, 10, 10);

		world.simulate(1);
		assert!(world.current_board_tiles_equals(expected_step1));
		world.simulate(1);
		assert!(world.current_board_tiles_equals(expected_step2));
		world.simulate(1);
		assert!(world.current_board_tiles_equals(expected_step3));
	}
}
