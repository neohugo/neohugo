//! Reader for the table blobs written by `tools/go-oracle/xtext-collate gen`.
//!
//! Format: magic `XTBLOB01`, then sections until EOF, each:
//! `u8 name_len, name, u8 width (1|2|4|8), u32 count, count * width LE bytes`.

pub(crate) struct Blob {
    sections: Vec<(&'static str, u8, &'static [u8])>,
}

impl Blob {
    pub(crate) fn parse(data: &'static [u8]) -> Blob {
        assert!(
            data.len() >= 8 && &data[..8] == b"XTBLOB01",
            "bad table blob"
        );
        let mut p = 8;
        let mut sections = Vec::new();
        while p < data.len() {
            let nl = data[p] as usize;
            p += 1;
            let name = std::str::from_utf8(&data[p..p + nl]).expect("blob section name");
            p += nl;
            let width = data[p];
            p += 1;
            let count =
                u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]) as usize;
            p += 4;
            let n = count * width as usize;
            sections.push((name, width, &data[p..p + n]));
            p += n;
        }
        Blob { sections }
    }

    fn get(&self, name: &str, width: u8) -> &'static [u8] {
        for &(n, w, d) in &self.sections {
            if n == name {
                assert_eq!(w, width, "blob section {name}: width {w}, want {width}");
                return d;
            }
        }
        panic!("blob section {name} missing");
    }

    pub(crate) fn bytes(&self, name: &str) -> &'static [u8] {
        self.get(name, 1)
    }

    pub(crate) fn str(&self, name: &str) -> &'static str {
        std::str::from_utf8(self.bytes(name)).expect("blob string")
    }

    pub(crate) fn u16s(&self, name: &str) -> Vec<u16> {
        self.get(name, 2)
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect()
    }

    pub(crate) fn u32s(&self, name: &str) -> Vec<u32> {
        self.get(name, 4)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    pub(crate) fn u64s(&self, name: &str) -> Vec<u64> {
        self.get(name, 8)
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect()
    }

    pub(crate) fn u32(&self, name: &str) -> u32 {
        let v = self.u32s(name);
        assert_eq!(v.len(), 1, "blob section {name}: not a scalar");
        v[0]
    }
}
