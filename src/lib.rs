/*!
# An implementation of the tftp packet

This implements only conversion into and from bytes for a tftp packet and includes some generic enums and a custom Error type.

## Example

```rust
use tftp_packet::Packet;
use tftp_packet::Mode;

let packet = Packet::RRQ{ filename: "test.txt".to_string(), mode: Mode::Octet };
let bytes = packet.clone().to_bytes();
assert_eq!(bytes, [0, 1, 116, 101, 115, 116, 46, 116, 120, 116, 0, 111, 99, 116, 101, 116, 0]);
assert_eq!(Packet::from_bytes(&bytes).unwrap(), packet);
```
*/
mod parsing;

use std::convert::TryFrom;
use std::{error::Error, fmt::Display};

use parsing::parse_block_number;
use parsing::parse_filename;
use parsing::parse_mode;
use parsing::take_u16;
use parsing::{parse_error_code, parse_error_message};

/// The error type for the tftp packet
#[derive(Debug, PartialEq)]
pub enum PacketError {
    /// General errors with an error message
    InvalidPacket(String),
    /// Invalid packet length with the expected length as field
    InvalidPacketLength(u16),
    /// Invalid Opcode with an error message
    InvalidOpcode(String),
}

impl Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PacketError::*;

        match self {
            InvalidPacket(s) => write!(f, "InvalidPacket: {}", s),
            InvalidOpcode(s) => write!(f, "InvalidOpcode: {}", s),
            InvalidPacketLength(s) => write!(f, "InvalidPacketLength: Expected {} bytes", s),
        }
    }
}

impl Error for PacketError {}

/// All tftp opcodes defined in rfc1350
///
/// ```
/// # use tftp_packet::Opcode;
/// assert_eq!(Opcode::RRQ as u16, 0);
/// assert_eq!(Opcode::try_from(1), Ok(Opcode::RRQ));
/// ```
#[derive(Debug, PartialEq, Clone)]
pub enum Opcode {
    RRQ,
    WRQ,
    DATA,
    ACK,
    ERROR,
}

impl TryFrom<u16> for Opcode {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Opcode::RRQ,
            2 => Opcode::WRQ,
            3 => Opcode::DATA,
            4 => Opcode::ACK,
            5 => Opcode::ERROR,
            _ => Err("Invalid opcode: {}")?,
        })
    }
}

impl TryFrom<&[u8; 2]> for Opcode {
    type Error = &'static str;

    fn try_from(value: &[u8; 2]) -> Result<Self, Self::Error> {
        let opcode = u16::from_be_bytes(*value);
        Self::try_from(opcode)
    }
}

/// All modes defined in rfc1350
///
/// ```
/// # use tftp_packet::Mode;
/// let mode: &str = Mode::Netascii.as_str();
/// assert_eq!(mode, "netascii");
/// assert_eq!(Mode::try_from("netascii"), Ok(Mode::Netascii));
/// ```
#[derive(Debug, PartialEq, Clone)]
pub enum Mode {
    Netascii,
    Octet,
    Mail,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

impl TryFrom<&str> for Mode {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "netascii" => Mode::Netascii,
            "octet" => Mode::Octet,
            "mail" => Mode::Mail,
            _ => Err("Invalid mode")?,
        })
    }
}

impl Into<&str> for &Mode {
    fn into(self) -> &'static str {
        match self {
            Mode::Netascii => "netascii",
            Mode::Octet => "octet",
            Mode::Mail => "mail",
        }
    }
}

/// All error codes defined in rfc1350 for an ERROR packet
///
/// ```
/// # use tftp_packet::ErrorCode;
/// assert_eq!(ErrorCode::NotDefined as u16, 0u16);
/// assert_eq!(ErrorCode::try_from(0), Ok(ErrorCode::NotDefined));
/// ```
#[derive(Debug, PartialEq, Clone)]
pub enum ErrorCode {
    NotDefined,
    FileNotFound,
    AccessViolation,
    DiskFull,
    IllegalOperation,
    UnknownTransferId,
    FileAlreadyExists,
    NoSuchUser,
}

impl TryFrom<u16> for ErrorCode {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => ErrorCode::NotDefined,
            1 => ErrorCode::FileNotFound,
            2 => ErrorCode::AccessViolation,
            3 => ErrorCode::DiskFull,
            4 => ErrorCode::IllegalOperation,
            5 => ErrorCode::UnknownTransferId,
            6 => ErrorCode::FileAlreadyExists,
            7 => ErrorCode::NoSuchUser,
            _ => Err("Invalid error code")?,
        })
    }
}

/// The tftp packet
#[derive(Debug, PartialEq, Clone)]
pub enum Packet {
    RRQ {
        filename: String,
        mode: Mode,
    },
    WRQ {
        filename: String,
        mode: Mode,
    },
    DATA {
        block_number: u16,
        data: Vec<u8>,
    },
    ACK {
        block_number: u16,
    },
    ERROR {
        error_code: ErrorCode,
        error_msg: String,
    },
}

impl Packet {
    /// Parse a packet from a byte array
    ///
    /// ```
    /// # use tftp_packet::Packet;
    /// # use tftp_packet::Mode;
    /// let packet = &[0u8, 1, 67, 68, 69, 0, 0x6f, 0x63, 0x74, 0x65, 0x74, 0];
    /// let packet = Packet::from_bytes(packet).unwrap();
    /// assert_eq!(packet, Packet::RRQ {
    ///    filename: "CDE".to_string(),
    ///    mode: Mode::Octet,
    /// });
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PacketError> {
        let (bytes, opcode_bytes) = take_u16(bytes).map_err(|_| {
            PacketError::InvalidOpcode("Error while parsing opcode. Opcode not a u15.".to_string())
        })?;

        let opcode = Opcode::try_from(opcode_bytes).map_err(|e| {
            PacketError::InvalidOpcode(format!("Error while parsing opcode: {}", e))
        })?;

        match opcode {
            Opcode::RRQ => {
                let (filename, bytes) = parse_filename(bytes)?;
                let (mode, _bytes) = parse_mode(bytes)?;

                Ok(Packet::RRQ { filename, mode })
            }
            Opcode::WRQ => {
                let (filename, bytes) = parse_filename(bytes)?;
                let (mode, _bytes) = parse_mode(bytes)?;

                Ok(Packet::WRQ { filename, mode })
            }
            Opcode::DATA => {
                let (block_number, bytes) = parse_block_number(bytes)?;

                let data = bytes.to_vec();

                if data.len() > 512 {
                    Err(PacketError::InvalidPacketLength(512))?
                }

                Ok(Packet::DATA { block_number, data })
            }
            Opcode::ACK => {
                let (block_number, bytes) = parse_block_number(bytes)?;

                if bytes.is_empty() {
                    Ok(Packet::ACK { block_number })
                } else {
                    Err(PacketError::InvalidPacketLength(4))?
                }
            }
            Opcode::ERROR => {
                let (error_code, bytes) = parse_error_code(bytes)?;
                let (error_msg, _bytes) = parse_error_message(bytes)?;

                Ok(Packet::ERROR {
                    error_code,
                    error_msg,
                })
            }
        }
    }

    /// Serialize a packet into a byte array
    ///
    /// ```
    /// # use tftp_packet::Packet;
    /// # use tftp_packet::Mode;
    /// let packet = Packet::RRQ {
    ///   filename: "CDE".to_string(),
    ///   mode: Mode::Octet,
    /// };
    /// let packet = packet.to_bytes();
    /// assert_eq!(packet, &[0u8, 1, 67, 68, 69, 0, 0x6f, 0x63, 0x74, 0x65, 0x74, 0]);
    /// ```
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            Packet::RRQ { filename, mode } => {
                let mut bytes = vec![0u8, 1];
                bytes.extend(filename.as_bytes());
                bytes.push(0);
                bytes.extend(Into::<&str>::into(&mode).as_bytes());
                bytes.push(0);
                bytes
            }
            Packet::WRQ { filename, mode } => {
                let mut bytes = vec![0u8, 2];
                bytes.extend(filename.as_bytes());
                bytes.push(0);
                bytes.extend(Into::<&str>::into(&mode).as_bytes());
                bytes.push(0);
                bytes
            }
            Packet::DATA { block_number, data } => {
                let mut bytes = vec![0u8, 3];
                bytes.extend(block_number.to_be_bytes().to_vec());
                bytes.extend(data);
                bytes
            }
            Packet::ACK { block_number } => {
                let mut bytes = vec![0u8, 4];
                bytes.extend(block_number.to_be_bytes().to_vec());
                bytes
            }
            Packet::ERROR {
                error_code,
                error_msg,
            } => {
                let mut bytes = vec![0u8, 5];
                bytes.extend((error_code as u16).to_be_bytes().to_vec());
                bytes.extend(error_msg.as_bytes());
                bytes.push(0);
                bytes
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode() {
        assert_eq!(Ok(Opcode::RRQ), 1.try_into());
        assert_eq!(Ok(Opcode::WRQ), 2.try_into());
        assert_eq!(Ok(Opcode::DATA), 3.try_into());
        assert_eq!(Ok(Opcode::ACK), 4.try_into());
        assert_eq!(Ok(Opcode::ERROR), 5.try_into());
    }

    #[test]
    fn test_rrq_parser() {
        let packet = &[0u8, 1, 67, 68, 69, 0, 0x6f, 0x63, 0x74, 0x65, 0x74, 0];
        let packet = Packet::from_bytes(packet).unwrap();
        assert_eq!(
            packet,
            Packet::RRQ {
                filename: "CDE".to_string(),
                mode: Mode::Octet,
            }
        );
    }

    #[test]
    fn test_wrq_parser() {
        let packet = &[0u8, 2, 67, 68, 69, 0, 0x6f, 0x63, 0x74, 0x65, 0x74, 0];
        let packet = Packet::from_bytes(packet).unwrap();
        assert_eq!(
            packet,
            Packet::WRQ {
                filename: "CDE".to_string(),
                mode: Mode::Octet,
            }
        );
    }

    #[test]
    fn test_data_parser() {
        let packet = &[0u8, 3, 0, 42, 67, 68, 69];
        let packet = Packet::from_bytes(packet).unwrap();
        assert_eq!(
            packet,
            Packet::DATA {
                block_number: 42,
                data: vec![67, 68, 69],
            }
        );
    }

    #[test]
    fn test_ack_parser() {
        let packet = &[0u8, 4, 0, 42];
        let packet = Packet::from_bytes(packet).unwrap();
        assert_eq!(packet, Packet::ACK { block_number: 42 });
    }

    #[test]
    fn test_error_parser() {
        let packet = &[0u8, 5, 0, 2, 67, 68, 69, 0];
        let packet = Packet::from_bytes(packet).unwrap();
        assert_eq!(
            packet,
            Packet::ERROR {
                error_code: ErrorCode::AccessViolation,
                error_msg: "CDE".to_string()
            }
        );
    }

    #[test]
    fn test_invalid_opcode() {
        let packet = &[0u8, 6, 67, 68, 69, 0, 0x6f, 0x63, 0x74, 0x65, 0x74, 0];
        assert!(matches!(
            Packet::from_bytes(packet),
            Err(PacketError::InvalidOpcode(..))
        ))
    }

    #[test]
    fn test_invalid_data_length() {
        let mut packet = vec![0u8, 3, 0, 42];
        packet.extend([69; 513].iter());
        assert!(matches!(
            Packet::from_bytes(&packet),
            Err(PacketError::InvalidPacketLength(..))
        ))
    }

    #[test]
    fn test_invalid_mode() {
        let packet = &[0u8, 1, 67, 68, 69, 0, 67, 0];
        assert!(matches!(
            Packet::from_bytes(packet),
            Err(PacketError::InvalidPacket(..))
        ))
    }

    #[test]
    fn test_rrq_to_bytes() {
        let packet = Packet::RRQ {
            filename: "CDE".to_string(),
            mode: Mode::Octet,
        };
        assert_eq!(
            packet.to_bytes(),
            vec![0u8, 1, 67, 68, 69, 0, 0x6f, 0x63, 0x74, 0x65, 0x74, 0]
        );
    }

    #[test]
    fn test_wrq_to_bytes() {
        let packet = Packet::WRQ {
            filename: "CDE".to_string(),
            mode: Mode::Octet,
        };
        assert_eq!(
            packet.to_bytes(),
            vec![0u8, 2, 67, 68, 69, 0, 0x6f, 0x63, 0x74, 0x65, 0x74, 0]
        );
    }

    #[test]
    fn test_data_to_bytes() {
        let packet = Packet::DATA {
            block_number: 42,
            data: vec![67, 68, 69],
        };
        assert_eq!(packet.to_bytes(), vec![0u8, 3, 0, 42, 67, 68, 69]);
    }

    #[test]
    fn test_ack_to_bytes() {
        let packet = Packet::ACK { block_number: 42 };
        assert_eq!(packet.to_bytes(), vec![0u8, 4, 0, 42]);
    }

    #[test]
    fn test_error_to_bytes() {
        let packet = Packet::ERROR {
            error_code: ErrorCode::AccessViolation,
            error_msg: "CDE".to_string(),
        };
        assert_eq!(packet.to_bytes(), vec![0u8, 5, 0, 2, 67, 68, 69, 0]);
    }
}
