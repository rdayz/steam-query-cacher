use super::QueryHeader;
use super::SourceQueryResponse;
use byteorder::{LittleEndian, ReadBytesExt};
use serde::Serialize;
use std::io::Cursor;
use std::io::Read;

#[derive(Debug, Clone, Serialize)]
#[repr(C)]
pub struct A2SRule {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct A2SRulesReply {
    pub header: QueryHeader,
    pub rules: Vec<A2SRule>,
    pub mods: Vec<Mod>,
}


impl SourceQueryResponse for A2SRulesReply {
    fn packet_header() -> QueryHeader {
        QueryHeader::A2SRulesReply
    }
}

impl Into<Vec<u8>> for A2SRulesReply {
    fn into(self) -> Vec<u8> {
        vec![]
    }
}

fn read_byte(c: &mut Cursor<Vec<u8>>) -> u8 {
    let byte = c.read_u8().unwrap();
    if byte == 1 {
        let byte2 = c.read_u8().unwrap();
        if byte2 == 1 {
            return 1;
        } else if byte2 == 2 {
            return 0;
        } else if byte2 == 3 {
            return 0xff;
        }
    }

    byte
}

fn read_uint4(c: &mut Cursor<Vec<u8>>) -> u32 {
    let buffer: Vec<u8> = vec![read_byte(c), read_byte(c), read_byte(c), read_byte(c)];
    let mut cursor = Cursor::new(buffer);
    cursor.read_u32::<LittleEndian>().unwrap()
}

fn read_string(c: &mut Cursor<Vec<u8>>) -> String {
    let len = read_byte(c);
    let mut buffer: Vec<u8> = Vec::default();
    for _ in 0..len {
        buffer.push(read_byte(c));
    }

    String::from_utf8_lossy(&buffer).to_string()
}

trait ReadCString {
    fn read_cstring(&mut self) -> String;
}

impl ReadCString for Cursor<Vec<u8>> {
    fn read_cstring(&mut self) -> String {
        let end = self.get_ref().len() as u64;
        let mut buf = [0; 1];
        let mut str_vec = Vec::with_capacity(256);
        while self.position() < end {
            self.read_exact(&mut buf).unwrap();
            if buf[0] == 0 {
                break;
            } else {
                str_vec.push(buf[0]);
            }
        }
        String::from_utf8_lossy(&str_vec[..]).into_owned()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Mod {
    pub id: u32,
    pub name: String,
}

impl TryFrom<&[u8]> for A2SRulesReply {
    type Error = std::io::Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let header = match QueryHeader::try_from(value[0]) {
            Ok(header) => header,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid header",
                ))
            }
        };

        if header != QueryHeader::A2SRulesReply {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid header for A2SRulesReply: {:?}", header),
            ));
        }

        let mut data: Cursor<Vec<u8>> = Cursor::new(value.to_vec());
        data.read_u8().unwrap();

        let count = data.read_u16::<LittleEndian>()?;
        let mut rules: Vec<A2SRule> = Vec::new();
        let mut payload: Vec<u8> = Vec::default();
        for _ in 0..count {
            if data.read_u8().unwrap() != 0
                && data.read_u8().unwrap() != 0
                && data.read_u8().unwrap() == 0
            {
                loop {
                    let byte = data.read_u8().unwrap();
                    if byte == 0 {
                        break;
                    }

                    payload.push(byte);
                }

                continue;
            } else {
                data.set_position(data.position() - 3);
            }

            rules.push(A2SRule {
                name: data.read_cstring(),
                value: data.read_cstring(),
            });
        }

        let mut test = Cursor::new(payload);
        read_byte(&mut test);
        read_byte(&mut test);

        let dlc1 = read_byte(&mut test);
        let dlc2 = read_byte(&mut test);
        if dlc1 != 0 {
            read_uint4(&mut test);
        }

        if dlc2 != 0 {
            read_uint4(&mut test);
        }

        let mut mods: Vec<Mod> = Vec::new();
        for _ in 0..read_byte(&mut test) {
            read_uint4(&mut test);

            let pos = test.position();
            let flag = read_byte(&mut test);
            if flag != 4 {
                test.set_position(pos);
            }

            mods.push(Mod {
                id: read_uint4(&mut test),
                name: read_string(&mut test),
            });
        }

        Ok(Self {
            header: QueryHeader::A2SRulesReply,
            rules,
            mods
        })
    }
}
