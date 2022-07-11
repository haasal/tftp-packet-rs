use std::str::from_utf8;

use nom::number::complete::be_u16;
use nom::{bytes::complete::*, IResult};

use crate::{ErrorCode, Mode, PacketError};

pub fn take_u16(input: &[u8]) -> IResult<&[u8], u16> {
    be_u16(input)
}

pub fn take_till_null(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_till(|c| c == 0)(input)
}

pub fn parse_filename(bytes: &[u8]) -> Result<(String, &[u8]), PacketError> {
    let (bytes, filename) = take_till_null(bytes)
        .map_err(|_| PacketError::InvalidPacket("Error while parsing filename".to_string()))?;

    let filename = from_utf8(filename)
        .map_err(|_| {
            PacketError::InvalidPacket(
                "Error while parsing filename. Not a valid UTF-8 string.".to_string(),
            )
        })?
        .to_string();

    Ok((filename, &bytes[1..]))
}

pub fn parse_mode(bytes: &[u8]) -> Result<(Mode, &[u8]), PacketError> {
    let (bytes, mode_bytes) = take_till_null(bytes)
        .map_err(|_| PacketError::InvalidPacket("Error while parsing mode".to_string()))?;

    let mode = from_utf8(mode_bytes).map_err(|_| {
        PacketError::InvalidPacket(
            "Error while parsing mode. Not a valid UTF-8 string.".to_string(),
        )
    })?;

    let mode: Mode = mode.try_into().map_err(|_| {
        PacketError::InvalidPacket("Error while parsing mode. Not a valid mode string.".to_string())
    })?;

    Ok((mode, bytes))
}

pub fn parse_block_number(bytes: &[u8]) -> Result<(u16, &[u8]), PacketError> {
    let (bytes, block_number) = take_u16(bytes).map_err(|_| {
        PacketError::InvalidPacket(
            "Error while parsing block number. Block number not a u16.".to_string(),
        )
    })?;

    Ok((block_number, bytes))
}

pub fn parse_error_code(bytes: &[u8]) -> Result<(ErrorCode, &[u8]), PacketError> {
    let (bytes, error_code) = take_u16(bytes).map_err(|_| {
        PacketError::InvalidPacket(
            "Error while parsing error code. Error code not a u16.".to_string(),
        )
    })?;

    let error_code = ErrorCode::try_from(error_code).map_err(|_| {
        PacketError::InvalidPacket(
            "Error while parsing error code. Error code not a valid error code.".to_string(),
        )
    })?;

    Ok((error_code, bytes))
}

pub fn parse_error_message(bytes: &[u8]) -> Result<(String, &[u8]), PacketError> {
    let (bytes, error_msg) = take_till_null(bytes)
        .map_err(|_| PacketError::InvalidPacket("Error while parsing error message".to_string()))?;

    let error_msg = from_utf8(error_msg)
        .map_err(|_| {
            PacketError::InvalidPacket(
                "Error while parsing error msg. Invalid UTF-8 string.".to_string(),
            )
        })?
        .to_string();

    Ok((error_msg, bytes))
}
