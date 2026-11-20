// Packet layouts adapted from mineflayer-plasmovoice (MIT).
// Copyright (c) 2023 Maksim Klimenko. See THIRD_PARTY_NOTICES.md.
// Verified against the published Plasmo Voice 2.1.17 binary (tools/ProtocolOracle.java).
use crate::{
    Result,
    wire::{MAX_COLLECTION, Reader, Writer},
};
use std::collections::HashMap;

pub const CHANNEL: &str = "plasmo:voice/v2";
pub const REGISTER: &[u8] = b"plasmo:voice/v2/installed\0plasmo:voice/v2/service\0plasmo:voice/v2";
pub const MAGIC: u32 = 0x4e9004e9;
pub const VERSION: &str = "2.1.17";

#[derive(Clone, Debug, PartialEq)]
pub struct Codec {
    pub name: String,
    pub params: HashMap<String, String>,
}
fn codec(r: &mut Reader<'_>) -> Result<Codec> {
    let name = r.utf()?;
    let mut params = HashMap::new();
    for _ in 0..r.count(64)? {
        params.insert(r.utf()?, r.utf()?);
    }
    Ok(Codec { name, params })
}
fn optional_codec(r: &mut Reader<'_>) -> Result<Option<Codec>> {
    if r.bool()? {
        Ok(Some(codec(r)?))
    } else {
        Ok(None)
    }
}
#[derive(Clone, Debug)]
pub struct Activation {
    pub id: u128,
    pub name: String,
    pub translation: String,
    pub icon: String,
    pub distances: Vec<u16>,
    pub dynamic_distance: bool,
    pub default_distance: u16,
    pub proximity: bool,
    pub transitive: bool,
    pub stereo_supported: bool,
    pub encoder: Option<Codec>,
    pub weight: i32,
}
fn activation(r: &mut Reader<'_>) -> Result<Activation> {
    let name = r.utf()?;
    let translation = r.utf()?;
    let icon = r.utf()?;
    let mut distances = Vec::new();
    let mut dynamic_distance = false;
    for index in 0..r.count(128)? {
        let d = r.i32()?;
        if index == 0 && d == -1 {
            dynamic_distance = true;
            continue;
        }
        if !(1..=32767).contains(&d) {
            return Err("invalid activation distance".into());
        }
        distances.push(d as u16);
    }
    let default = r.i32()?;
    if !(0..=32767).contains(&default) {
        return Err("invalid default distance".into());
    }
    let proximity = r.bool()?;
    let transitive = r.bool()?;
    let stereo_supported = r.bool()?;
    let encoder = optional_codec(r)?;
    let weight = r.i32()?;
    Ok(Activation {
        id: 0,
        name,
        translation,
        icon,
        distances,
        dynamic_distance,
        default_distance: default as u16,
        proximity,
        transitive,
        stereo_supported,
        encoder,
        weight,
    })
}
#[derive(Clone, Debug)]
pub struct Profile {
    pub id: u128,
    pub name: String,
    pub properties: Vec<[String; 3]>,
}
fn profile(r: &mut Reader<'_>) -> Result<Profile> {
    let id = r.uuid()?;
    let name = r.utf()?;
    let mut properties = Vec::new();
    for _ in 0..r.count(128)? {
        properties.push([r.utf()?, r.utf()?, r.utf()?]);
    }
    Ok(Profile {
        id,
        name,
        properties,
    })
}
#[derive(Clone, Debug)]
pub struct SourceLine {
    pub id: u128,
    pub name: String,
    pub translation: String,
    pub icon: String,
    pub default_volume: f32,
    pub weight: i32,
    pub players: Option<Vec<Profile>>,
}
fn source_line(r: &mut Reader<'_>) -> Result<SourceLine> {
    let name = r.utf()?;
    let translation = r.utf()?;
    let icon = r.utf()?;
    let volume = r.f64()?;
    if !volume.is_finite() || !(0.0..=100.0).contains(&volume) {
        return Err("invalid source line volume".into());
    }
    let weight = r.i32()?;
    let players = if r.bool()? {
        let mut players = Vec::new();
        for _ in 0..r.count(MAX_COLLECTION)? {
            players.push(profile(r)?);
        }
        Some(players)
    } else {
        None
    };
    Ok(SourceLine {
        id: 0,
        name,
        translation,
        icon,
        default_volume: volume as f32,
        weight,
        players,
    })
}
#[derive(Clone, Debug)]
pub struct VoicePlayer {
    pub id: u128,
    pub name: String,
    pub server_muted: bool,
    pub voice_disabled: bool,
    pub microphone_muted: bool,
}
fn voice_player(r: &mut Reader<'_>) -> Result<VoicePlayer> {
    Ok(VoicePlayer {
        id: r.uuid()?,
        name: r.utf()?,
        server_muted: r.bool()?,
        voice_disabled: r.bool()?,
        microphone_muted: r.bool()?,
    })
}
fn position(r: &mut Reader<'_>) -> Result<[f64; 3]> {
    let point = [r.f64()?, r.f64()?, r.f64()?];
    if point.iter().any(|v| !v.is_finite()) {
        return Err("invalid source position".into());
    }
    Ok(point)
}
fn permissions(r: &mut Reader<'_>) -> Result<HashMap<String, bool>> {
    let mut values = HashMap::new();
    for _ in 0..r.count(MAX_COLLECTION)? {
        values.insert(r.utf()?, r.bool()?);
    }
    Ok(values)
}
#[derive(Clone, Debug)]
pub struct Config {
    pub server: u128,
    pub sample_rate: u32,
    pub mtu: usize,
    pub encoder: Option<Codec>,
    pub encryption: Option<(String, Vec<u8>)>,
    pub activations: Vec<Activation>,
    pub lines: Vec<SourceLine>,
    pub permissions: HashMap<String, bool>,
    pub hidden_icons: Vec<String>,
    pub icon_offset: [f64; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub enum SourceKind {
    Player,
    Entity(i32),
    Static {
        position: [f64; 3],
        direction: [f64; 3],
    },
    Direct {
        relative: Option<[f64; 3]>,
        camera_relative: bool,
    },
}
#[derive(Clone, Debug)]
pub struct Source {
    pub id: u128,
    pub addon: String,
    pub line: u128,
    pub kind: SourceKind,
    pub player: u128,
    pub name: String,
    pub state: u8,
    pub stereo: bool,
    pub muted: bool,
    pub icon_visible: bool,
    pub angle: i32,
    pub player_info: Option<VoicePlayer>,
}
fn source(r: &mut Reader<'_>) -> Result<Source> {
    let kind = r.utf()?;
    let addon = r.utf()?;
    let id = r.uuid()?;
    let mut name = if r.bool()? { r.utf()? } else { String::new() };
    let state = r.u8()?;
    let decoder = optional_codec(r)?;
    if decoder.is_some_and(|c| !c.name.eq_ignore_ascii_case("opus")) {
        return Err("unsupported source codec".into());
    }
    let stereo = r.bool()?;
    let line = r.uuid()?;
    let icon_visible = r.bool()?;
    let angle = r.i32()?;
    let mut player_info = None;
    let mut player = 0;
    let mut muted = false;
    let kind = match kind.as_str() {
        "PLAYER" => {
            let info = voice_player(r)?;
            player = info.id;
            if name.is_empty() {
                name = info.name.clone();
            }
            muted = info.server_muted || info.voice_disabled || info.microphone_muted;
            player_info = Some(info);
            SourceKind::Player
        }
        "ENTITY" => SourceKind::Entity(r.i32()?),
        "STATIC" => SourceKind::Static {
            position: position(r)?,
            direction: position(r)?,
        },
        "DIRECT" => {
            if r.bool()? {
                let sender = profile(r)?;
                player = sender.id;
                if name.is_empty() {
                    name = sender.name;
                }
            }
            let relative = if r.bool()? { Some(position(r)?) } else { None };
            SourceKind::Direct {
                relative,
                camera_relative: r.bool()?,
            }
        }
        _ => return Err("unsupported source kind".into()),
    };
    Ok(Source {
        id,
        addon,
        line,
        kind,
        player,
        name,
        state,
        stereo,
        muted,
        icon_visible,
        angle,
        player_info,
    })
}
#[derive(Clone, Debug)]
pub enum Control {
    PlayerInfoRequest,
    Connection {
        secret: u128,
        host: String,
        port: u16,
    },
    Config(Config),
    Permissions(HashMap<String, bool>),
    Language {
        name: String,
        values: HashMap<String, String>,
    },
    Players(Vec<VoicePlayer>),
    PlayerUpdate(VoicePlayer),
    PlayerDisconnect(u128),
    ActivationRegister(Activation),
    ActivationUnregister(u128),
    LineRegister(SourceLine),
    LineUnregister(u128),
    LinePlayers {
        line: u128,
        players: Vec<Profile>,
    },
    LinePlayerAdd {
        line: u128,
        player: Profile,
    },
    LinePlayerRemove {
        line: u128,
        player: u128,
    },
    SelfSource {
        source: Source,
        player: u128,
        activation: u128,
        sequence: u64,
    },
    DistanceVisualize {
        radius: i32,
        color: u32,
        position: Option<[f64; 3]>,
    },
    ActionBar(String),
    Source(Source),
    SourceEnd {
        source: u128,
        sequence: u64,
    },
    Unknown(u8),
}

pub fn read_control(data: &[u8]) -> Result<Control> {
    let mut r = Reader::new(data)?;
    let id = r.u8()?;
    let packet = match id {
        1 => {
            let secret = r.uuid()?;
            let host = r.utf()?;
            let port = r.i32()?;
            if !(1..=65535).contains(&port) {
                return Err("invalid voice port".into());
            }
            Control::Connection {
                secret,
                host,
                port: port as u16,
            }
        }
        2 => Control::PlayerInfoRequest,
        3 => {
            let server = r.uuid()?;
            let sample_rate = r.u32()?;
            if ![8000, 12000, 16000, 24000, 48000].contains(&sample_rate) {
                return Err("unsupported capture rate".into());
            }
            let mtu = r.count(4096)?;
            if mtu < 64 {
                return Err("MTU too small".into());
            }
            let encoder = optional_codec(&mut r)?;
            let encryption = if r.bool()? {
                Some((r.utf()?, r.bytes(4096)?))
            } else {
                None
            };
            let mut lines = Vec::new();
            for _ in 0..r.count(128)? {
                lines.push(source_line(&mut r)?);
            }
            let mut activations = Vec::new();
            for _ in 0..r.count(128)? {
                activations.push(activation(&mut r)?);
            }
            let permissions = permissions(&mut r)?;
            // PlayerIconConfig: visibility modes and a three-double offset.
            let mut hidden_icons = Vec::new();
            for _ in 0..r.count(32)? {
                hidden_icons.push(r.utf()?);
            }
            let icon_offset = position(&mut r)?;
            Control::Config(Config {
                server,
                sample_rate,
                mtu,
                encoder,
                encryption,
                activations,
                lines,
                permissions,
                hidden_icons,
                icon_offset,
            })
        }
        4 => Control::Permissions(permissions(&mut r)?),
        6 => {
            let name = r.utf()?;
            let mut values = HashMap::new();
            for _ in 0..r.count(MAX_COLLECTION)? {
                values.insert(r.utf()?, r.utf()?);
            }
            Control::Language { name, values }
        }
        7 => {
            let mut players = Vec::new();
            for _ in 0..r.count(MAX_COLLECTION)? {
                players.push(voice_player(&mut r)?);
            }
            Control::Players(players)
        }
        8 => Control::PlayerUpdate(voice_player(&mut r)?),
        9 => Control::PlayerDisconnect(r.uuid()?),
        14 => Control::DistanceVisualize {
            radius: r.i32()?,
            color: r.u32()?,
            position: if r.bool()? {
                Some(position(&mut r)?)
            } else {
                None
            },
        },
        16 => Control::Source(source(&mut r)?),
        17 => Control::SelfSource {
            source: source(&mut r)?,
            player: r.uuid()?,
            activation: r.uuid()?,
            sequence: r.u64()?,
        },
        18 => Control::SourceEnd {
            source: r.uuid()?,
            sequence: r.u64()?,
        },
        19 => Control::ActivationRegister(activation(&mut r)?),
        20 => Control::ActivationUnregister(r.uuid()?),
        21 => Control::LineRegister(source_line(&mut r)?),
        22 => Control::LineUnregister(r.uuid()?),
        23 => Control::LinePlayerAdd {
            line: r.uuid()?,
            player: profile(&mut r)?,
        },
        24 => Control::LinePlayerRemove {
            line: r.uuid()?,
            player: r.uuid()?,
        },
        25 => {
            let line = r.uuid()?;
            let mut players = Vec::new();
            for _ in 0..r.count(MAX_COLLECTION)? {
                players.push(profile(&mut r)?);
            }
            Control::LinePlayers { line, players }
        }
        26 => Control::ActionBar(r.utf()?),
        _ => return Ok(Control::Unknown(id)),
    };
    r.finish()?;
    Ok(packet)
}

pub fn player_info(
    version: &str,
    public_key: &[u8],
    disabled: bool,
    muted: bool,
) -> Result<Vec<u8>> {
    if public_key.len() > 1024 {
        return Err("public key too large".into());
    }
    let mut w = Writer::default();
    w.u8(10);
    w.bool(disabled);
    w.bool(muted);
    w.utf(version)?;
    w.utf(VERSION)?;
    w.bytes(public_key);
    Ok(w.0)
}
pub fn player_state(disabled: bool, muted: bool) -> Vec<u8> {
    vec![11, disabled.into(), muted.into()]
}
pub fn language_request(language: &str) -> Result<Vec<u8>> {
    let mut w = Writer::default();
    w.u8(5);
    w.utf(language)?;
    Ok(w.0)
}
pub fn activation_distances(values: &[(u128, u16)]) -> Vec<u8> {
    let mut w = Writer::default();
    w.u8(13);
    w.i32(values.len() as i32);
    for (id, distance) in values {
        w.uuid(*id);
        w.i32(i32::from(*distance));
    }
    w.0
}
pub fn source_request(source: u128) -> Vec<u8> {
    let mut w = Writer::default();
    w.u8(15);
    w.uuid(source);
    w.0
}
pub fn player_end(sequence: u64, activation: u128, distance: u16) -> Vec<u8> {
    let mut w = Writer::default();
    w.u8(12);
    w.u64(sequence);
    w.uuid(activation);
    w.u16(distance);
    w.0
}
fn header(id: u8, secret: u128, timestamp: u64) -> Writer {
    let mut w = Writer::default();
    w.i32(MAGIC as i32);
    w.u8(id);
    w.uuid(secret);
    w.u64(timestamp);
    w
}
pub fn ping(secret: u128, timestamp: u64, server: Option<(&str, u16)>) -> Result<Vec<u8>> {
    let mut w = header(1, secret, timestamp);
    w.u64(timestamp);
    if let Some((host, port)) = server {
        w.utf(host)?;
        w.u16(port);
    }
    Ok(w.0)
}
pub fn player_audio(
    secret: u128,
    timestamp: u64,
    sequence: u64,
    data: &[u8],
    activation: u128,
    distance: u16,
) -> Vec<u8> {
    let mut w = header(2, secret, timestamp);
    w.u64(sequence);
    w.bytes(data);
    w.uuid(activation);
    w.u16(distance);
    w.bool(false);
    w.0
}
#[derive(Debug)]
pub enum Datagram {
    Ping {
        timestamp: u64,
    },
    Audio {
        sequence: u64,
        data: Vec<u8>,
        source: u128,
        state: u8,
        distance: u16,
    },
    Unknown,
}
pub fn read_datagram(data: &[u8], expected_secret: u128) -> Result<Datagram> {
    if data.len() > 8192 {
        return Err("UDP packet too large".into());
    }
    let mut r = Reader::new(data)?;
    if r.u32()? != MAGIC {
        return Err("invalid UDP magic".into());
    }
    let id = r.u8()?;
    if r.uuid()? != expected_secret {
        return Err("wrong UDP session".into());
    }
    r.u64()?;
    let result = match id {
        1 => {
            let timestamp = r.u64()?;
            Datagram::Ping { timestamp }
        }
        3 => Datagram::Audio {
            sequence: r.u64()?,
            data: r.bytes(4144)?,
            source: r.uuid()?,
            state: r.u8()?,
            distance: r.u16()?,
        },
        _ => return Ok(Datagram::Unknown),
    };
    r.finish()?;
    Ok(result)
}
