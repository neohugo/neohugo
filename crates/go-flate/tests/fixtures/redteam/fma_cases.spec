# FMA-sensitive block decisions: inputs where the float32 EstimatedBits in
# writeBlockDynamic (token.go:203) decides between reusing the previous
# Huffman table and starting a new one, and the decision depends on which of
# its multiply-adds are fused. arm64 Go fuses all three (mFastLog2's FMADDS
# and FNMSUBS, the shannon FMADDS); amd64 Go fuses none. Found with a
# targeted search (PORTING.md, "FMA sensitivity"); input of `oracle specs`.
# Fields: wrapper level data dict ops. The comment after each case lists the
# variants that write different bytes than arm64 Go (and the Rust port):
# "amd64" (nothing fused), "-log2a" (only mFastLog2's FMADDS unfused),
# "-log2b" (only its FNMSUBS unfused), "-acc" (only the shannon FMADDS
# unfused).
flate 5 gen:15:48714:742362+gen:8:16821:748747+gen:8:28318:162465+gen:15:37217:1068 none W131070,C
#   amd64 -acc
flate 3 gen:3:25220:229648+gen:3:40315:447813+gen:3:15286:81960+gen:3:50249:690944 none W131070,C
#   amd64 -acc
flate 3 gen:8:23404:19803+gen:8:42131:839438+gen:8:44128:623727+gen:8:21407:480530 none W131070,C
#   amd64 -acc
flate 2 gen:8:38786:848031+gen:5:26749:831527+gen:5:13733:61885+gen:8:51802:927915 none W131070,C
#   -log2a
flate 3 gen:8:38507:861232+gen:8:27028:257236+gen:8:30452:177271+gen:8:35083:184642 none W131070,C
#   amd64 -acc
flate 4 gen:8:41679:713450+gen:3:23856:12541+gen:3:25227:492164+gen:8:40308:414397 none W131070,C
#   amd64 -log2b -acc
flate 5 gen:15:25090:13713+gen:8:40445:198435+gen:8:45400:611731+gen:15:20135:835754 none W131070,C
#   -log2a -log2b
flate 5 gen:7:23736:562188+gen:8:41799:947607+gen:8:25852:586480+gen:7:39683:231683 none W131070,C
#   amd64 -log2b -acc
flate 5 gen:8:35363:316699+gen:4:30172:214049+gen:4:34391:460019+gen:8:31144:946508 none W131070,C
#   amd64 -log2a
flate 5 gen:8:38793:107492+gen:11:26742:462991+gen:11:25714:143637+gen:8:39821:875090 none W131070,C
#   -log2b
flate 6 gen:3:36142:478780+gen:8:29393:288823+gen:8:34011:284123+gen:3:31524:845518 none W131070,C
#   amd64 -acc
flate 5 gen:15:47730:408578+gen:5:17805:915644+gen:5:15082:315182+gen:15:50453:31805 none W131070,C
#   amd64 -acc
flate 2 gen:3:46675:914212+gen:3:18860:264794+gen:3:55443:76703+gen:3:10092:484232 none W131070,C
#   amd64 -log2a -acc
flate 4 gen:3:18752:518981+gen:3:46783:15111+gen:3:56257:52454+gen:3:9278:226217 none W131070,C
#   -log2a
flate 5 gen:3:38169:626608+gen:8:27366:730614+gen:8:33729:808019+gen:3:31806:312459 none W131070,C
#   amd64 -log2a -log2b
flate 5 gen:7:17685:676190+gen:8:47850:5157+gen:8:30478:103513+gen:7:35057:164001 none W131070,C
#   amd64 -log2a
