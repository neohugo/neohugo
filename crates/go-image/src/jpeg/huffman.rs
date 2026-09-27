//! Port of Go 1.27.1 `image/jpeg/huffman.go`.
//!
//! The bit-reading methods operate on [`Input`] (the decoder's `r`, `bits`
//! and `bytes` fields, split out of the decoder struct so that the Huffman
//! tables can be borrowed while bits are consumed).

use super::Decoder;
use super::{ERR_MISSING_FF00, ERR_SHORT_HUFFMAN_DATA, Error, Input, MAX_TC, MAX_TH};

// maxCodeLength is the maximum (inclusive) number of bits in a Huffman code.
pub(crate) const MAX_CODE_LENGTH: usize = 16;

// maxNCodes is the maximum (inclusive) number of codes in a Huffman tree.
pub(crate) const MAX_N_CODES: usize = 256;

// lutSize is the log-2 size of the Huffman decoder's look-up table.
pub(crate) const LUT_SIZE: u32 = 8;

/// huffman is a Huffman decoder, specified in section C.
///
/// Go: image/jpeg/huffman.go:huffman
#[derive(Clone)]
pub(crate) struct Huffman {
    // length is the number of codes in the tree.
    pub(crate) n_codes: i32,
    // lut is the look-up table for the next lutSize bits in the bit-stream.
    // The high 8 bits of the uint16 are the encoded value. The low 8 bits
    // are 1 plus the code length, or 0 if the value is too large to fit in
    // lutSize bits.
    pub(crate) lut: [u16; 1 << LUT_SIZE],
    // vals are the decoded values, sorted by their encoding.
    pub(crate) vals: [u8; MAX_N_CODES],
    // minCodes[i] is the minimum code of length i, or -1 if there are no
    // codes of that length.
    pub(crate) min_codes: [i32; MAX_CODE_LENGTH],
    // maxCodes[i] is the maximum code of length i, or -1 if there are no
    // codes of that length.
    pub(crate) max_codes: [i32; MAX_CODE_LENGTH],
    // valsIndices[i] is the index into vals of minCodes[i].
    pub(crate) vals_indices: [i32; MAX_CODE_LENGTH],
}

impl Default for Huffman {
    fn default() -> Self {
        Huffman {
            n_codes: 0,
            lut: [0; 1 << LUT_SIZE],
            vals: [0; MAX_N_CODES],
            min_codes: [0; MAX_CODE_LENGTH],
            max_codes: [0; MAX_CODE_LENGTH],
            vals_indices: [0; MAX_CODE_LENGTH],
        }
    }
}

impl Input<'_> {
    /// ensureNBits reads bytes from the byte buffer to ensure that d.bits.n is
    /// at least n. For best performance (avoiding function calls inside hot
    /// loops), the caller is the one responsible for first checking that
    /// d.bits.n < n.
    ///
    /// Go: image/jpeg/huffman.go:decoder.ensureNBits
    #[inline(never)]
    pub(crate) fn ensure_n_bits(&mut self, n: i32) -> Result<(), Error> {
        loop {
            let c = match self.read_byte_stuffed_byte() {
                Ok(c) => c,
                Err(Error::UnexpectedEof) => {
                    return Err(Error::Format(ERR_SHORT_HUFFMAN_DATA));
                }
                Err(e) => return Err(e),
            };
            self.bits.a = self.bits.a << 8 | c as u32;
            self.bits.n += 8;
            if self.bits.m == 0 {
                self.bits.m = 1 << 7;
            } else {
                self.bits.m <<= 8;
            }
            if self.bits.n >= n {
                break;
            }
        }
        Ok(())
    }

    /// receiveExtend is the composition of RECEIVE and EXTEND, specified in
    /// section F.2.2.1.
    ///
    /// Go: image/jpeg/huffman.go:decoder.receiveExtend
    #[inline]
    pub(crate) fn receive_extend(&mut self, t: u8) -> Result<i32, Error> {
        if self.bits.n < t as i32 {
            self.ensure_n_bits(t as i32)?;
        }
        self.bits.n -= t as i32;
        self.bits.m = crate::go_shr_u32(self.bits.m, t as u32);
        let s = crate::go_shl_i32(1, t as u32);
        let mut x =
            (crate::go_shr_u32(self.bits.a, self.bits.n as u8 as u32) as i32) & s.wrapping_sub(1);

        // This adjustment, assuming two's complement, is a branchless equivalent of:
        //
        // if x < s>>1 {
        //   x += ((-1) << t) + 1
        // }
        //
        // sign is either -1 or 0, depending on whether x is in the low or high
        // half of the range 0 .. 1<<t.
        let sign = crate::go_shr_i32(x, t.wrapping_sub(1) as u32).wrapping_sub(1);
        x = x.wrapping_add(sign & crate::go_shl_i32(-1, t as u32).wrapping_add(1));

        Ok(x)
    }

    /// decodeHuffman returns the next Huffman-coded value from the bit-stream,
    /// decoded according to h.
    ///
    /// Go: image/jpeg/huffman.go:decoder.decodeHuffman
    #[inline]
    pub(crate) fn decode_huffman(&mut self, h: &Huffman) -> Result<u8, Error> {
        if h.n_codes == 0 {
            return Err(Error::Format("uninitialized Huffman table"));
        }

        let mut slow_path = false;
        if self.bits.n < 8 {
            if let Err(err) = self.ensure_n_bits(8) {
                if !err.is_format(ERR_MISSING_FF00) && !err.is_format(ERR_SHORT_HUFFMAN_DATA) {
                    return Err(err);
                }
                // There are no more bytes of data in this segment, but we may still
                // be able to read the next symbol out of the previously read bits.
                // First, undo the readByte that the ensureNBits call made.
                if self.bytes.n_unreadable != 0 {
                    self.unread_byte_stuffed_byte();
                }
                slow_path = true;
            }
        }
        if !slow_path {
            let v =
                h.lut[((self.bits.a >> (self.bits.n - LUT_SIZE as i32) as u32) & 0xff) as usize];
            if v != 0 {
                let n = (v & 0xff) - 1;
                self.bits.n -= n as i32;
                self.bits.m = crate::go_shr_u32(self.bits.m, n as u32);
                return Ok((v >> 8) as u8);
            }
        }

        // slowPath:
        let mut code = 0i32;
        for i in 0..MAX_CODE_LENGTH {
            if self.bits.n == 0 {
                self.ensure_n_bits(1)?;
            }
            if self.bits.a & self.bits.m != 0 {
                code |= 1;
            }
            self.bits.n -= 1;
            self.bits.m >>= 1;
            if code <= h.max_codes[i] {
                return Ok(h.vals[(h.vals_indices[i] + code - h.min_codes[i]) as usize]);
            }
            code <<= 1;
        }
        Err(Error::Format("bad Huffman code"))
    }

    // Go: image/jpeg/huffman.go:decoder.decodeBit
    #[inline]
    pub(crate) fn decode_bit(&mut self) -> Result<bool, Error> {
        if self.bits.n == 0 {
            self.ensure_n_bits(1)?;
        }
        let ret = self.bits.a & self.bits.m != 0;
        self.bits.n -= 1;
        self.bits.m >>= 1;
        Ok(ret)
    }

    // Go: image/jpeg/huffman.go:decoder.decodeBits
    #[inline]
    pub(crate) fn decode_bits(&mut self, n: i32) -> Result<u32, Error> {
        if self.bits.n < n {
            self.ensure_n_bits(n)?;
        }
        let mut ret = crate::go_shr_u32(self.bits.a, (self.bits.n - n) as u32);
        ret &= crate::go_shl_u32(1, n as u32).wrapping_sub(1);
        self.bits.n -= n;
        self.bits.m = crate::go_shr_u32(self.bits.m, n as u32);
        Ok(ret)
    }
}

impl Decoder<'_> {
    /// processDHT processes a Define Huffman Table marker, and initializes a
    /// huffman struct from its contents. Specified in section B.2.4.2.
    ///
    /// Go: image/jpeg/huffman.go:decoder.processDHT
    pub(crate) fn process_dht(&mut self, mut n: i64) -> Result<(), Error> {
        while n > 0 {
            if n < 17 {
                return Err(Error::Format("DHT has wrong length"));
            }
            self.inp.read_full(&mut self.tmp[..17])?;
            let tc = self.tmp[0] >> 4;
            if tc as usize > MAX_TC {
                return Err(Error::Format("bad Tc value"));
            }
            let th = self.tmp[0] & 0x0f;
            // The baseline th <= 1 restriction is specified in table B.5.
            if th as usize > MAX_TH || (self.baseline && th > 1) {
                return Err(Error::Format("bad Th value"));
            }
            let h = &mut self.huff[tc as usize][th as usize];

            // Read nCodes and h.vals (and derive h.nCodes).
            // nCodes[i] is the number of codes with code length i.
            // h.nCodes is the total number of codes.
            h.n_codes = 0;
            let mut n_codes = [0i32; MAX_CODE_LENGTH];
            for i in 0..MAX_CODE_LENGTH {
                n_codes[i] = self.tmp[i + 1] as i32;
                h.n_codes += n_codes[i];
            }
            if h.n_codes == 0 {
                return Err(Error::Format("Huffman table has zero length"));
            }
            if h.n_codes > MAX_N_CODES as i32 {
                return Err(Error::Format("Huffman table has excessive length"));
            }
            n -= h.n_codes as i64 + 17;
            if n < 0 {
                return Err(Error::Format("DHT has wrong length"));
            }
            let nc = h.n_codes as usize;
            self.inp.read_full(&mut h.vals[..nc])?;

            // Derive the look-up table.
            h.lut = [0; 1 << LUT_SIZE];
            let (mut x, mut code) = (0u32, 0u32);
            for i in 0..LUT_SIZE {
                code <<= 1;
                for _j in 0..n_codes[i as usize] {
                    // The codeLength is 1+i, so shift code by 8-(1+i) to
                    // calculate the high bits for every 8-bit sequence
                    // whose codeLength's high bits matches code.
                    // The high 8 bits of lutValue are the encoded value.
                    // The low 8 bits are 1 plus the codeLength.
                    let base = (code << (7 - i)) as u8;
                    let lut_value = (h.vals[x as usize] as u16) << 8 | (2 + i) as u16;
                    for k in 0..(1u32 << (7 - i)) {
                        h.lut[(base | k as u8) as usize] = lut_value;
                    }
                    code = code.wrapping_add(1);
                    x += 1;
                }
            }

            // Derive minCodes, maxCodes, and valsIndices.
            let (mut c, mut index) = (0i32, 0i32);
            for (i, &n) in n_codes.iter().enumerate() {
                if n == 0 {
                    h.min_codes[i] = -1;
                    h.max_codes[i] = -1;
                    h.vals_indices[i] = -1;
                } else {
                    h.min_codes[i] = c;
                    h.max_codes[i] = c + n - 1;
                    h.vals_indices[i] = index;
                    c += n;
                    index += n;
                }
                c <<= 1;
            }
        }
        Ok(())
    }
}
