
from math import *

c_freq = 64
freq_multiplier = 2**(1/12.)
for octave in range(1,16):
	c_freq_octave = c_freq * (2**(octave-1))
	
	note_freq = c_freq_octave
	for note in range(12):
		print(note_freq)
		note_freq *= freq_multiplier
