#file = open("energizersound.txt")
#filedata = file.read()
energizersound = "20 3 1A 3 17 3 16 3 15 3 13 3 10 3"
duplicatedsound = "30 02 32 02 34 02 35 02  37 02"
duplicatefailedsound = "18 1 16 1"
playerhurt = "10  01 20 01 13 01 23 01"
nums = [int(num, 16) for num in playerhurt.split()]
#nums.pop(0)
print(nums)

octave_notes = ["c", "c#", "d", "d#", "e", "f", "f#", "g", "g#", "a", "a#", "b"]
mul_chars = "tsiqhw"

out_notes = ""
curr_octave = 3
curr_mul = 1
for i in range(0, len(nums), 2):
	code = nums[i]
	mul = nums[i + 1]
	
	if mul != curr_mul:
		mulindex = 1
		while 2**(mulindex + 1) < mul:
			mulindex += 1
		mul_char = mul_chars[mulindex]
		if 2**mulindex == mul:
			out_notes += mul_char
		elif int(2**mulindex * 1.5) == mul:
			out_notes += mul_char
			out_notes += "."
		else:
			# Check for triplets
			curr_pow_2 = 1
			while 2**curr_pow_2 < mul * 3:
				curr_pow_2 += 1
			if mul * 3 == 2**curr_pow_2:
				out_notes += mul_chars[curr_pow_2]
				out_notes += "3"
			else:
				raise "Not implemented"
		
		curr_mul = mul
	
	if code == 0:
		out_notes = "x"
	elif code < 240:
		octave = code // 16
		note = code % 16
		while octave < curr_octave:
			out_notes += "-"
			curr_octave -= 1
		while octave > curr_octave:
			out_notes += "+"
			curr_octave += 1
		out_notes += octave_notes[note]
		#print(code, mul, octave, note, octave_notes[note])
	else:
		out_notes += str(code - 240)

print(out_notes)
