# Go 1.27.1 behaviours that look like bugs but must be reproduced byte for byte.
# Input of `oracle specs` (fields: wrapper level data dict ops).
#
# NewWriterDict at levels 7-9 leaves blockStart at 0 after fillWindow, so the
# first block's stored-block fallback (incompressible data, small dictionary)
# also stores the dictionary bytes: the stream inflates to dict+data.
flate 7 gen:1:5000:11 gen:1:10:12 W5000,C
flate 8 gen:1:5000:13 gen:1:10:14 W5000,C
flate 9 gen:1:5000:15 gen:1:10:16 W5000,C
flate 9 gen:1:100000:17 gen:1:10:18 W100000,C
flate 9 gen:1:2000:19 gen:1:1:20 W1000,F,W1000,C
zlib 9 gen:1:5000:21 gen:1:10:22 W5000,C
flate 9 gen:1:5000:23 gen:1:10:24 W2500,C,R,W2500,C
# Same with larger dictionaries (Huffman-only / stored decisions differ).
flate 9 gen:1:5000:25 gen:1:1000:26 W5000,C
flate 7 gen:1:30000:27 gen:1:40000:28 W30000,C
