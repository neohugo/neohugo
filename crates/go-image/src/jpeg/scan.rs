//! Port of Go 1.27.1 `image/jpeg/scan.go`.

use super::dct::{BLOCK_SIZE, Block, idct};
use super::{
    AC_TABLE, DC_TABLE, Decoder, Error, Input, MAX_COMPONENTS, MAX_TH, RST0_MARKER, RST7_MARKER,
    UNZIG,
};
use crate::geom::rect;
use crate::image::Gray;
use crate::ycbcr::{YCbCr, YCbCrSubsampleRatio};

#[derive(Clone, Copy, Default)]
struct ScanComp {
    comp_index: u8,
    td: u8, // DC table selector.
    ta: u8, // AC table selector.
}

impl Decoder<'_> {
    /// makeImg allocates and initializes the destination image.
    ///
    /// Go: image/jpeg/scan.go:decoder.makeImg
    fn make_img(&mut self, mxx: i64, myy: i64) {
        if self.n_comp == 1 {
            let m = Gray::new(rect(0, 0, 8 * mxx, 8 * myy));
            self.img1 = Some(m.into_sub_image(rect(0, 0, self.width, self.height)));
            return;
        }

        // Determine if we need flex mode for non-standard subsampling.
        // Flex mode is needed when:
        // - Cb and Cr have different sampling factors, or
        // - The Y component doesn't have the maximum sampling factors, or
        // - The ratio doesn't match any standard YCbCrSubsampleRatio.
        let mut subsample_ratio = YCbCrSubsampleRatio::Ratio444;
        if self.comp[1].h != self.comp[2].h
            || self.comp[1].v != self.comp[2].v
            || self.max_h != self.comp[0].h
            || self.max_v != self.comp[0].v
        {
            self.flex = true;
        } else {
            let h_ratio = self.max_h / self.comp[1].h;
            let v_ratio = self.max_v / self.comp[1].v;
            match h_ratio << 4 | v_ratio {
                0x11 => subsample_ratio = YCbCrSubsampleRatio::Ratio444,
                0x12 => subsample_ratio = YCbCrSubsampleRatio::Ratio440,
                0x21 => subsample_ratio = YCbCrSubsampleRatio::Ratio422,
                0x22 => subsample_ratio = YCbCrSubsampleRatio::Ratio420,
                0x41 => subsample_ratio = YCbCrSubsampleRatio::Ratio411,
                0x42 => subsample_ratio = YCbCrSubsampleRatio::Ratio410,
                _ => self.flex = true,
            }
        }

        let m = YCbCr::new(
            rect(0, 0, 8 * self.max_h * mxx, 8 * self.max_v * myy),
            subsample_ratio,
        );
        self.img3 = Some(m.into_sub_image(rect(0, 0, self.width, self.height)));

        if self.n_comp == 4 {
            let (h3, v3) = (self.comp[3].h, self.comp[3].v);
            self.black_pix = Some(vec![0u8; (8 * h3 * mxx * 8 * v3 * myy) as usize]);
            self.black_stride = 8 * h3 * mxx;
        }
    }

    /// Specified in section B.2.3.
    ///
    /// Go: image/jpeg/scan.go:decoder.processSOS
    pub(crate) fn process_sos(&mut self, n: i64) -> Result<(), Error> {
        if self.n_comp == 0 {
            return Err(Error::Format("missing SOF marker"));
        }
        if n < 6 || 4 + 2 * self.n_comp < n || n % 2 != 0 {
            return Err(Error::Format("SOS has wrong length"));
        }
        self.inp.read_full(&mut self.tmp[..n as usize])?;
        let n_comp = self.tmp[0] as usize;
        if n != 4 + 2 * n_comp as i64 {
            return Err(Error::Format(
                "SOS length inconsistent with number of components",
            ));
        }
        let mut scan = [ScanComp::default(); MAX_COMPONENTS];
        let mut total_hv = 0i64;
        for i in 0..n_comp {
            let cs = self.tmp[1 + 2 * i]; // Component selector.
            let mut comp_index: i64 = -1;
            for (j, comp) in self.comp[..self.n_comp as usize].iter().enumerate() {
                if cs == comp.c {
                    comp_index = j as i64;
                }
            }
            if comp_index < 0 {
                return Err(Error::Format("unknown component selector"));
            }
            scan[i].comp_index = comp_index as u8;
            // Section B.2.3 states that "the value of Cs_j shall be different from
            // the values of Cs_1 through Cs_(j-1)". Since we have previously
            // verified that a frame's component identifiers (C_i values in section
            // B.2.2) are unique, it suffices to check that the implicit indexes
            // into d.comp are unique.
            for j in 0..i {
                if scan[i].comp_index == scan[j].comp_index {
                    return Err(Error::Format("repeated component selector"));
                }
            }
            total_hv += self.comp[comp_index as usize].h * self.comp[comp_index as usize].v;

            // The baseline t <= 1 restriction is specified in table B.3.
            scan[i].td = self.tmp[2 + 2 * i] >> 4;
            let t = scan[i].td;
            if t as usize > MAX_TH || (self.baseline && t > 1) {
                return Err(Error::Format("bad Td value"));
            }
            scan[i].ta = self.tmp[2 + 2 * i] & 0x0f;
            let t = scan[i].ta;
            if t as usize > MAX_TH || (self.baseline && t > 1) {
                return Err(Error::Format("bad Ta value"));
            }
        }
        // Section B.2.3 states that if there is more than one component then the
        // total H*V values in a scan must be <= 10.
        if self.n_comp > 1 && total_hv > 10 {
            return Err(Error::Format("total sampling factors too large"));
        }

        // zigStart and zigEnd are the spectral selection bounds.
        // ah and al are the successive approximation high and low values.
        // The spec calls these values Ss, Se, Ah and Al.
        //
        // For sequential JPEGs, these parameters are hard-coded to 0/63/0/0, as
        // per table B.3.
        let (mut zig_start, mut zig_end, mut ah, mut al) =
            (0i32, (BLOCK_SIZE - 1) as i32, 0u32, 0u32);
        if self.progressive {
            zig_start = self.tmp[1 + 2 * n_comp] as i32;
            zig_end = self.tmp[2 + 2 * n_comp] as i32;
            ah = (self.tmp[3 + 2 * n_comp] >> 4) as u32;
            al = (self.tmp[3 + 2 * n_comp] & 0x0f) as u32;
            if (zig_start == 0 && zig_end != 0)
                || zig_start > zig_end
                || BLOCK_SIZE as i32 <= zig_end
            {
                return Err(Error::Format("bad spectral selection bounds"));
            }
            if zig_start != 0 && n_comp != 1 {
                return Err(Error::Format(
                    "progressive AC coefficients for more than one component",
                ));
            }
            if ah != 0 && ah != al + 1 {
                return Err(Error::Format("bad successive approximation values"));
            }
        }

        // mxx and myy are the number of MCUs (Minimum Coded Units) in the image.
        // The MCU dimensions are based on the maximum sampling factors.
        // For standard subsampling, maxH/maxV equals h0/v0 (Y's factors).
        // For flex mode, Y may not have the maximum factors.
        let mxx = (self.width + 8 * self.max_h - 1) / (8 * self.max_h);
        let myy = (self.height + 8 * self.max_v - 1) / (8 * self.max_v);
        if self.img1.is_none() && self.img3.is_none() {
            self.make_img(mxx, myy);
        }
        if self.progressive {
            for i in 0..n_comp {
                let comp_index = scan[i].comp_index as usize;
                if self.prog_coeffs[comp_index].is_none() {
                    self.prog_coeffs[comp_index] = Some(vec![
                        [0i32; BLOCK_SIZE];
                        (mxx * myy * self.comp[comp_index].h * self.comp[comp_index].v)
                            as usize
                    ]);
                }
            }
        }

        self.inp.bits = Default::default();
        let (mut mcu, mut expected_rst) = (0i64, RST0_MARKER);
        // b is the decoded coefficients, in natural (not zig-zag) order.
        let mut b: Block;
        let mut dc = [0i32; MAX_COMPONENTS];
        // bx and by are the location of the current block, in units of 8x8
        // blocks: the third block in the first row has (bx, by) = (2, 0).
        let (mut bx, mut by): (i64, i64);
        let mut block_count = 0i64;
        for my in 0..myy {
            for mx in 0..mxx {
                for i in 0..n_comp {
                    let comp_index = scan[i].comp_index as usize;
                    let hi = self.comp[comp_index].h;
                    let vi = self.comp[comp_index].v;
                    for j in 0..hi * vi {
                        // The blocks are traversed one MCU at a time. For 4:2:0 chroma
                        // subsampling, there are four Y 8x8 blocks in every 16x16 MCU.
                        //
                        // For a sequential 32x16 pixel image, the Y blocks visiting order is:
                        //	0 1 4 5
                        //	2 3 6 7
                        //
                        // For progressive images, the interleaved scans (those with nComp > 1)
                        // are traversed as above, but non-interleaved scans are traversed left
                        // to right, top to bottom:
                        //	0 1 2 3
                        //	4 5 6 7
                        // Only DC scans (zigStart == 0) can be interleaved. AC scans must have
                        // only one component.
                        //
                        // To further complicate matters, for non-interleaved scans, there is no
                        // data for any blocks that are inside the image at the MCU level but
                        // outside the image at the pixel level. For example, a 24x16 pixel 4:2:0
                        // progressive image consists of two 16x16 MCUs. The interleaved scans
                        // will process 8 Y blocks:
                        //	0 1 4 5
                        //	2 3 6 7
                        // The non-interleaved scans will process only 6 Y blocks:
                        //	0 1 2
                        //	3 4 5
                        if n_comp != 1 {
                            bx = hi * mx + j % hi;
                            by = vi * my + j / hi;
                        } else {
                            let q = mxx * hi;
                            bx = block_count % q;
                            by = block_count / q;
                            block_count += 1;
                            if bx * 8 >= self.width || by * 8 >= self.height {
                                continue;
                            }
                        }

                        // Load the previous partially decoded coefficients, if applicable.
                        if self.progressive {
                            b = self.prog_coeffs[comp_index].as_ref().unwrap()
                                [(by * mxx * hi + bx) as usize];
                        } else {
                            b = [0; BLOCK_SIZE];
                        }

                        if ah != 0 {
                            self.refine(
                                &mut b,
                                scan[i].ta as usize,
                                zig_start,
                                zig_end,
                                1i32 << al,
                            )?;
                        } else {
                            let mut zig = zig_start;
                            if zig == 0 {
                                zig += 1;
                                // Decode the DC coefficient, as specified in section F.2.2.1.
                                let value = self
                                    .inp
                                    .decode_huffman(&self.huff[DC_TABLE][scan[i].td as usize])?;
                                if value > 16 {
                                    return Err(Error::Unsupported("excessive DC component"));
                                }
                                let dc_delta = self.inp.receive_extend(value)?;
                                dc[comp_index] = dc[comp_index].wrapping_add(dc_delta);
                                b[0] = crate::go_shl_i32(dc[comp_index], al);
                            }

                            if zig <= zig_end && self.eob_run > 0 {
                                self.eob_run -= 1;
                            } else {
                                // Decode the AC coefficients, as specified in section F.2.2.2.
                                let huff = &self.huff[AC_TABLE][scan[i].ta as usize];
                                while zig <= zig_end {
                                    let value = self.inp.decode_huffman(huff)?;
                                    let val0 = value >> 4;
                                    let val1 = value & 0x0f;
                                    if val1 != 0 {
                                        zig += val0 as i32;
                                        if zig > zig_end {
                                            break;
                                        }
                                        let ac = self.inp.receive_extend(val1)?;
                                        b[UNZIG[zig as usize]] = crate::go_shl_i32(ac, al);
                                    } else {
                                        if val0 != 0x0f {
                                            self.eob_run = 1u16 << val0;
                                            if val0 != 0 {
                                                let bits = self.inp.decode_bits(val0 as i32)?;
                                                self.eob_run |= bits as u16;
                                            }
                                            self.eob_run = self.eob_run.wrapping_sub(1);
                                            break;
                                        }
                                        zig += 0x0f;
                                    }
                                    zig += 1;
                                }
                            }
                        }

                        if self.progressive {
                            // Save the coefficients.
                            self.prog_coeffs[comp_index].as_mut().unwrap()
                                [(by * mxx * hi + bx) as usize] = b;
                            // At this point, we could call reconstructBlock to dequantize and perform the
                            // inverse DCT, to save early stages of a progressive image to the *image.YCbCr
                            // buffers (the whole point of progressive encoding), but in Go, the jpeg.Decode
                            // function does not return until the entire image is decoded, so we "continue"
                            // here to avoid wasted computation. Instead, reconstructBlock is called on each
                            // accumulated block by the reconstructProgressiveImage method after all of the
                            // SOS markers are processed.
                            continue;
                        }
                        self.reconstruct_block(&mut b, bx, by, comp_index)?;
                    } // for j
                } // for i
                mcu += 1;
                if self.ri > 0 && mcu % self.ri == 0 && mcu < mxx * myy {
                    // For well-formed input, the RST[0-7] restart marker follows
                    // immediately. For corrupt input, call findRST to try to
                    // resynchronize.
                    self.inp.read_full(&mut self.tmp[..2])?;
                    if self.tmp[0] != 0xff || self.tmp[1] != expected_rst {
                        self.find_rst(expected_rst)?;
                    }
                    expected_rst += 1;
                    if expected_rst == RST7_MARKER + 1 {
                        expected_rst = RST0_MARKER;
                    }
                    // Reset the Huffman decoder.
                    self.inp.bits = Default::default();
                    // Reset the DC components, as per section F.2.1.3.1.
                    dc = [0; MAX_COMPONENTS];
                    // Reset the progressive decoder state, as per section G.1.2.2.
                    self.eob_run = 0;
                }
            } // for mx
        } // for my

        Ok(())
    }

    /// refine decodes a successive approximation refinement block, as
    /// specified in section G.1.2. `ta` selects `d.huff[acTable][ta]`.
    ///
    /// Go: image/jpeg/scan.go:decoder.refine
    fn refine(
        &mut self,
        b: &mut Block,
        ta: usize,
        zig_start: i32,
        zig_end: i32,
        delta: i32,
    ) -> Result<(), Error> {
        // Refining a DC component is trivial.
        if zig_start == 0 {
            if zig_end != 0 {
                panic!("unreachable");
            }
            let bit = self.inp.decode_bit()?;
            if bit {
                b[0] |= delta;
            }
            return Ok(());
        }

        let h = &self.huff[AC_TABLE][ta];
        // Refining AC components is more complicated; see sections G.1.2.2 and G.1.2.3.
        let mut zig = zig_start;
        if self.eob_run == 0 {
            while zig <= zig_end {
                let mut z = 0i32;
                let value = self.inp.decode_huffman(h)?;
                let val0 = value >> 4;
                let val1 = value & 0x0f;

                match val1 {
                    0 => {
                        if val0 != 0x0f {
                            self.eob_run = 1u16 << val0;
                            if val0 != 0 {
                                let bits = self.inp.decode_bits(val0 as i32)?;
                                self.eob_run |= bits as u16;
                            }
                            break;
                        }
                    }
                    1 => {
                        z = delta;
                        let bit = self.inp.decode_bit()?;
                        if !bit {
                            z = -z;
                        }
                    }
                    _ => return Err(Error::Format("unexpected Huffman code")),
                }

                zig = refine_non_zeroes(&mut self.inp, b, zig, zig_end, val0 as i32, delta)?;
                if zig > zig_end {
                    return Err(Error::Format("too many coefficients"));
                }
                if z != 0 {
                    b[UNZIG[zig as usize]] = z;
                }
                zig += 1;
            }
        }
        if self.eob_run > 0 {
            self.eob_run -= 1;
            refine_non_zeroes(&mut self.inp, b, zig, zig_end, -1, delta)?;
        }
        Ok(())
    }

    // Go: image/jpeg/scan.go:decoder.reconstructProgressiveImage
    pub(crate) fn reconstruct_progressive_image(&mut self) -> Result<(), Error> {
        // The mxx, by and bx variables have the same meaning as in the
        // processSOS method.
        let mxx = (self.width + 8 * self.max_h - 1) / (8 * self.max_h);
        for i in 0..self.n_comp as usize {
            if self.prog_coeffs[i].is_none() {
                continue;
            }
            let v = 8 * self.max_v / self.comp[i].v;
            let h = 8 * self.max_h / self.comp[i].h;
            let stride = mxx * self.comp[i].h;
            let mut by = 0i64;
            while by * v < self.height {
                let mut bx = 0i64;
                while bx * h < self.width {
                    let idx = (by * stride + bx) as usize;
                    // Go passes &d.progCoeffs[i][idx]; the block is copied out,
                    // reconstructed in place and stored back.
                    let mut blk = self.prog_coeffs[i].as_ref().unwrap()[idx];
                    let r = self.reconstruct_block(&mut blk, bx, by, i);
                    self.prog_coeffs[i].as_mut().unwrap()[idx] = blk;
                    r?;
                    bx += 1;
                }
                by += 1;
            }
        }
        Ok(())
    }

    /// reconstructBlock dequantizes, performs the inverse DCT and stores the
    /// block to the image.
    ///
    /// Go: image/jpeg/scan.go:decoder.reconstructBlock
    pub(crate) fn reconstruct_block(
        &mut self,
        b: &mut Block,
        mut bx: i64,
        mut by: i64,
        comp_index: usize,
    ) -> Result<(), Error> {
        let qt = &self.quant[self.comp[comp_index].tq as usize];
        for zig in 0..BLOCK_SIZE {
            b[UNZIG[zig]] = b[UNZIG[zig]].wrapping_mul(qt[zig]);
        }
        idct(b);

        let (mut h, mut v) = (0i64, 0i64);
        if self.flex {
            // Flex mode: scale bx and by according to the component's sampling factors.
            h = self.comp[comp_index].expand_h;
            v = self.comp[comp_index].expand_v;
            (bx, by) = (bx * h, by * v);
        }

        let (dst, stride): (&mut [u8], i64) = if self.n_comp == 1 {
            let img1 = self.img1.as_mut().unwrap();
            let s = img1.stride;
            (&mut img1.pix[(8 * (by * s + bx)) as usize..], s)
        } else {
            match comp_index {
                0 => {
                    let img3 = self.img3.as_mut().unwrap();
                    let s = img3.y_stride;
                    (&mut img3.y[(8 * (by * s + bx)) as usize..], s)
                }
                1 => {
                    let img3 = self.img3.as_mut().unwrap();
                    let s = img3.c_stride;
                    (&mut img3.cb[(8 * (by * s + bx)) as usize..], s)
                }
                2 => {
                    let img3 = self.img3.as_mut().unwrap();
                    let s = img3.c_stride;
                    (&mut img3.cr[(8 * (by * s + bx)) as usize..], s)
                }
                3 => {
                    let s = self.black_stride;
                    let bp = self.black_pix.as_mut().unwrap();
                    (&mut bp[(8 * (by * s + bx)) as usize..], s)
                }
                _ => return Err(Error::Unsupported("too many components")),
            }
        };

        if self.flex {
            // Flex mode: expand each source pixel to h×v destination pixels.
            for y in 0..8i64 {
                let y8 = y * 8;
                let yv = y * v;
                for x in 0..8i64 {
                    let val = 0.max(255.min(b[(y8 + x) as usize].wrapping_add(128))) as u8;
                    let xh = x * h;
                    for yy in 0..v {
                        for xx in 0..h {
                            dst[((yv + yy) * stride + xh + xx) as usize] = val;
                        }
                    }
                }
            }
            return Ok(());
        }

        // Level shift by +128, clip to [0, 255], and write to dst.
        for y in 0..8i64 {
            let y8 = y * 8;
            let y_stride = y * stride;
            for x in 0..8i64 {
                dst[(y_stride + x) as usize] =
                    0.max(255.min(b[(y8 + x) as usize].wrapping_add(128))) as u8;
            }
        }
        Ok(())
    }

    /// findRST advances past the next RST restart marker that matches
    /// expectedRST. Other than I/O errors, it is also an error if we encounter
    /// an {0xFF, M} two-byte marker sequence where M is not 0x00, 0xFF or the
    /// expectedRST.
    ///
    /// Precondition: d.tmp[:2] holds the next two bytes of JPEG-encoded input
    /// (input in the d.readFull sense).
    ///
    /// Go: image/jpeg/scan.go:decoder.findRST
    fn find_rst(&mut self, expected_rst: u8) -> Result<(), Error> {
        loop {
            // i is the index such that, at the bottom of the loop, we read 2-i
            // bytes into d.tmp[i:2], maintaining the invariant that d.tmp[:2]
            // holds the next two bytes of JPEG-encoded input. It is either 0 or 1,
            // so that each iteration advances by 1 or 2 bytes (or returns).
            let mut i = 0usize;

            if self.tmp[0] == 0xff {
                if self.tmp[1] == expected_rst {
                    return Ok(());
                } else if self.tmp[1] == 0xff {
                    i = 1;
                } else if self.tmp[1] != 0x00 {
                    // libjpeg's jdmarker.c's jpeg_resync_to_restart does something
                    // fancy here, treating RST markers within two (modulo 8) of
                    // expectedRST differently from RST markers that are 'more
                    // distant'. Until we see evidence that recovering from such
                    // cases is frequent enough to be worth the complexity, we take
                    // a simpler approach for now. Any marker that's not 0x00, 0xff
                    // or expectedRST is a fatal FormatError.
                    return Err(Error::Format("bad RST marker"));
                }
            } else if self.tmp[1] == 0xff {
                self.tmp[0] = 0xff;
                i = 1;
            }

            self.inp.read_full(&mut self.tmp[i..2])?;
        }
    }
}

/// refineNonZeroes refines non-zero entries of b in zig-zag order. If nz >= 0,
/// the first nz zero entries are skipped over.
///
/// Go: image/jpeg/scan.go:decoder.refineNonZeroes
#[inline]
fn refine_non_zeroes(
    inp: &mut Input<'_>,
    b: &mut Block,
    mut zig: i32,
    zig_end: i32,
    mut nz: i32,
    delta: i32,
) -> Result<i32, Error> {
    while zig <= zig_end {
        let u = UNZIG[zig as usize];
        if b[u] == 0 {
            if nz == 0 {
                break;
            }
            nz -= 1;
            zig += 1;
            continue;
        }
        let bit = inp.decode_bit()?;
        if !bit {
            zig += 1;
            continue;
        }
        if b[u] >= 0 {
            b[u] = b[u].wrapping_add(delta);
        } else {
            b[u] = b[u].wrapping_sub(delta);
        }
        zig += 1;
    }
    Ok(zig)
}
