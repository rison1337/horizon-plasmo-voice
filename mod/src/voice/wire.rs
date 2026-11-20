//! Java DataInput/DataOutput primitives. All lengths are checked before allocation.
use crate::Result;

pub const MAX_CONTROL: usize = 1 << 20;
pub const MAX_COLLECTION: usize = 1024;

pub struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        if data.len() > MAX_CONTROL {
            return Err("control packet too large".into());
        }
        Ok(Self { data })
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.data.len() {
            return Err("truncated packet".into());
        }
        let (value, rest) = self.data.split_at(n);
        self.data = rest;
        Ok(value)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn bool(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("invalid boolean".into()),
        }
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(self.i32()? as u32)
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn uuid(&mut self) -> Result<u128> {
        Ok(u128::from_be_bytes(self.take(16)?.try_into().unwrap()))
    }
    pub fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.u64()?))
    }
    pub fn count(&mut self, max: usize) -> Result<usize> {
        let n = usize::try_from(self.i32()?).map_err(|_| "negative length")?;
        if n > max {
            return Err("length limit exceeded".into());
        }
        Ok(n)
    }
    pub fn bytes(&mut self, max: usize) -> Result<Vec<u8>> {
        let n = self.count(max)?;
        Ok(self.take(n)?.to_vec())
    }
    pub fn utf(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        let mut bytes = self.take(n)?;
        let mut units = Vec::with_capacity(n);
        while !bytes.is_empty() {
            let a = bytes[0];
            let (unit, used) = if (1..=127).contains(&a) {
                (u16::from(a), 1)
            } else if a & 0xe0 == 0xc0 && bytes.len() >= 2 && bytes[1] & 0xc0 == 0x80 {
                let unit = (u16::from(a & 31) << 6) | u16::from(bytes[1] & 63);
                if unit != 0 && unit < 128 {
                    return Err("overlong modified UTF-8".into());
                }
                (unit, 2)
            } else if a & 0xf0 == 0xe0
                && bytes.len() >= 3
                && bytes[1] & 0xc0 == 0x80
                && bytes[2] & 0xc0 == 0x80
            {
                let unit = (u16::from(a & 15) << 12)
                    | (u16::from(bytes[1] & 63) << 6)
                    | u16::from(bytes[2] & 63);
                if unit < 2048 {
                    return Err("overlong modified UTF-8".into());
                }
                (unit, 3)
            } else {
                return Err("invalid modified UTF-8".into());
            };
            units.push(unit);
            bytes = &bytes[used..];
        }
        String::from_utf16(&units).map_err(|_| "invalid UTF-16 string".into())
    }
    pub fn finish(self) -> Result<()> {
        if self.data.is_empty() {
            Ok(())
        } else {
            Err("unexpected trailing bytes".into())
        }
    }
}

#[derive(Default)]
pub struct Writer(pub Vec<u8>);
impl Writer {
    pub fn u8(&mut self, x: u8) {
        self.0.push(x);
    }
    pub fn bool(&mut self, x: bool) {
        self.u8(u8::from(x));
    }
    pub fn u16(&mut self, x: u16) {
        self.0.extend(x.to_be_bytes());
    }
    pub fn i32(&mut self, x: i32) {
        self.0.extend(x.to_be_bytes());
    }
    pub fn u64(&mut self, x: u64) {
        self.0.extend(x.to_be_bytes());
    }
    pub fn uuid(&mut self, x: u128) {
        self.0.extend(x.to_be_bytes());
    }
    pub fn bytes(&mut self, x: &[u8]) {
        self.i32(x.len().try_into().expect("bounded packet"));
        self.0.extend(x);
    }
    pub fn utf(&mut self, x: &str) -> Result<()> {
        let mut bytes = Vec::new();
        for unit in x.encode_utf16() {
            if (1..128).contains(&unit) {
                bytes.push(unit as u8);
            } else if unit < 2048 {
                bytes.extend([0xc0 | (unit >> 6) as u8, 0x80 | (unit & 63) as u8]);
            } else {
                bytes.extend([
                    0xe0 | (unit >> 12) as u8,
                    0x80 | ((unit >> 6) & 63) as u8,
                    0x80 | (unit & 63) as u8,
                ]);
            }
        }
        self.u16(bytes.len().try_into().map_err(|_| "string too long")?);
        self.0.extend(bytes);
        Ok(())
    }
}
