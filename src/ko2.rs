//! Talks to the KO II over USB-C MIDI on a background thread, so the window never freezes.
//!
//! - Pads: normal MIDI notes (official TE guide).
//! - "Read pads": READ-ONLY SysEx file requests (community docs, see sysex.rs):
//!   GREET, FILE INIT, METADATA GET, LIST. Nothing is ever written to the device.

use crate::pads::{self, GROUPS};
use crate::sysex::{self, Reply};
use eframe::egui;
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The MIDI output to the KO II, shared with the drummer thread. None = not connected.
#[derive(Clone, Default)]
pub struct MidiOut(Arc<Mutex<Option<MidiOutputConnection>>>);

impl MidiOut {
    /// Send one MIDI message. Returns false if not connected or sending failed.
    pub fn send(&self, msg: &[u8]) -> bool {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(out) => out.send(msg).is_ok(),
            None => false,
        }
    }

    fn set(&self, conn: Option<MidiOutputConnection>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = conn;
    }
}

/// Commands from the window to the MIDI thread.
pub enum Command {
    PadDown(u8),
    PadUp(u8),
    ReadPads,
}

/// News from the MIDI thread to the window.
pub enum Event {
    Connected(String),
    Disconnected,
    /// The KO II is plugged in but we could not open it (another app may be using it).
    Busy(String),
    ReadProgress(String),
    ReadDone(ReadResult),
    ReadFailed(String),
    /// A pad was pressed (on = true) or let go on the KO II itself: its MIDI note.
    DeviceNote { note: u8, on: bool },
    /// A group (0 = A) was picked on the KO II itself.
    DeviceGroup(usize),
    /// The sound on a pad was changed on the KO II itself (new sample slot).
    DevicePadSound { project: u8, pad: pads::PadRef, slot: u16 },
    /// Any other message from the KO II, in words (shown in the status bar, also in the log file).
    DeviceMessage(String),
}

/// Everything the KO II sends is written to `%APPDATA%\koii-ep133\midi-log.txt`,
/// so we can learn which buttons and knobs it reports. Cleared when the app starts.
pub mod midi_log {
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::time::Instant;

    static START: OnceLock<Instant> = OnceLock::new();

    fn path() -> Option<PathBuf> {
        Some(PathBuf::from(std::env::var_os("APPDATA")?).join("koii-ep133").join("midi-log.txt"))
    }

    pub fn clear() {
        START.get_or_init(Instant::now);
        if let Some(p) = path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, "");
        }
    }

    pub fn write(line: &str) {
        let Some(p) = path() else { return };
        let secs = START.get_or_init(Instant::now).elapsed().as_secs_f32();
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{secs:9.3}  {line}");
        }
    }

    pub fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
    }
}

/// A normal (not SysEx) MIDI message in plain words, e.g. "ch 1 note 43 on vel 100".
fn describe(bytes: &[u8]) -> String {
    let ch = (bytes[0] & 0x0F) + 1;
    match (bytes[0] & 0xF0, bytes.get(1), bytes.get(2)) {
        (0x90, Some(n), Some(0)) | (0x80, Some(n), _) => format!("ch {ch} note {n} off"),
        (0x90, Some(n), Some(v)) => format!("ch {ch} note {n} on vel {v}"),
        (0xB0, Some(cc), Some(v)) => format!("ch {ch} CC {cc} = {v}"),
        (0xC0, Some(p), _) => format!("ch {ch} program {p}"),
        (0xE0, Some(lo), Some(hi)) => format!("ch {ch} pitch bend {}", (*hi as i32) << 7 | *lo as i32),
        (0xA0, Some(n), Some(v)) => format!("ch {ch} aftertouch note {n} = {v}"),
        (0xD0, Some(v), _) => format!("ch {ch} pressure {v}"),
        _ => format!("other: {}", midi_log::hex(bytes)),
    }
}

pub struct ReadResult {
    pub project: u8,
    pub os_version: Option<String>,
    /// Sample slot per pad (4 groups x 12 pads, PAD_LABELS order). None = 0 / unknown.
    pub slots: Vec<Vec<Option<u16>>>,
    /// Sample slot -> name, from the /sounds list.
    pub sound_names: HashMap<u16, String>,
}

pub struct Ko2 {
    commands: Sender<Command>,
    events: Receiver<Event>,
    pub out: MidiOut,
}

impl Ko2 {
    pub fn start(ctx: egui::Context) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel();
        let out = MidiOut::default();
        let worker_out = out.clone();
        std::thread::Builder::new()
            .name("ko2-midi".into())
            .spawn(move || Worker::new(cmd_rx, ev_tx, ctx, worker_out).run())
            .expect("start MIDI thread");
        Self { commands: cmd_tx, events: ev_rx, out }
    }

    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    pub fn poll(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

/// Port names that look like the KO II. Windows shows the USB name of the device.
fn is_ko2_port(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    n.contains("EP-133") || n.contains("EP133") || n.contains("K.O. II") || n.contains("KO II")
}

struct Connection {
    port_name: String,
    out: MidiOut,
    _input: MidiInputConnection<()>,
    sysex: Receiver<Vec<u8>>,
    next_id: u16,
}

struct Worker {
    commands: Receiver<Command>,
    events: Sender<Event>,
    ctx: egui::Context,
    out: MidiOut,
    conn: Option<Connection>,
    last_busy: Option<String>,
}

/// Errors while reading. `Debug` means the KO II reported a firmware error.
enum ReadError {
    Debug(String),
    Other(String),
}

impl From<String> for ReadError {
    fn from(s: String) -> Self {
        ReadError::Other(s)
    }
}

impl Worker {
    fn new(commands: Receiver<Command>, events: Sender<Event>, ctx: egui::Context, out: MidiOut) -> Self {
        Self { commands, events, ctx, out, conn: None, last_busy: None }
    }

    fn emit(&self, event: Event) {
        let _ = self.events.send(event);
        self.ctx.request_repaint();
    }

    fn run(mut self) {
        let mut next_check = Instant::now();
        loop {
            match self.commands.recv_timeout(Duration::from_millis(30)) {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return, // window closed
            }
            self.check_device_events();
            if Instant::now() >= next_check {
                self.check_connection();
                next_check = Instant::now() + Duration::from_secs(1);
            }
        }
    }

    fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::PadDown(note) => self.send_midi(&[0x90, note, 100]),
            Command::PadUp(note) => self.send_midi(&[0x80, note, 0]),
            Command::ReadPads => self.read_pads(),
        }
    }

    fn send_midi(&mut self, msg: &[u8]) {
        if self.conn.is_some() && !self.out.send(msg) {
            self.drop_connection();
        }
    }

    fn drop_connection(&mut self) {
        if self.conn.take().is_some() {
            self.out.set(None);
            self.emit(Event::Disconnected);
        }
    }

    fn ko2_output_port_name() -> Option<String> {
        let out = MidiOutput::new("koii-scan").ok()?;
        out.ports()
            .iter()
            .filter_map(|p| out.port_name(p).ok())
            .find(|name| is_ko2_port(name))
    }

    fn check_connection(&mut self) {
        let found = Self::ko2_output_port_name();
        match (&self.conn, found) {
            (Some(conn), Some(name)) if conn.port_name == name => {}
            (Some(_), _) => self.drop_connection(),
            (None, Some(name)) => match open(&name, &self.out, self.events.clone(), self.ctx.clone()) {
                Ok(mut conn) => {
                    self.last_busy = None;
                    // FILE INIT (read-only, same as "Read pads") so the KO II tells us
                    // when a group is picked on the device. If it fails, notes still work.
                    let _ = request(&mut conn, sysex::CMD_FILE, &[0x01, 0x01, 0x00, 0x40, 0x00, 0x00]);
                    self.conn = Some(conn);
                    self.emit(Event::Connected(name));
                }
                Err(msg) => {
                    if self.last_busy.as_deref() != Some(msg.as_str()) {
                        self.last_busy = Some(msg.clone());
                        self.emit(Event::Busy(msg));
                    }
                }
            },
            (None, None) => {
                if self.last_busy.take().is_some() {
                    self.emit(Event::Disconnected);
                }
            }
        }
    }

    /// Messages the KO II sends by itself: group picked on the device.
    fn check_device_events(&mut self) {
        let Some(conn) = &self.conn else { return };
        let mut group = None;
        while let Ok(frame) = conn.sysex.try_recv() {
            let shown = &frame[..frame.len().min(120)];
            midi_log::write(&format!("sysex {} bytes: {}", frame.len(), midi_log::hex(shown)));
            if let Some((fid, json)) = sysex::parse_metadata_event(&frame) {
                midi_log::write(&format!("  = file {fid} changed: {json}"));
                let active = json.get("active").and_then(|v| v.as_u64());
                if let Some(g) = active.and_then(|a| pads::group_from_event(fid, a)) {
                    group = Some(g);
                    let _ = self.events.send(Event::DeviceMessage(format!("picked group {}", GROUPS[g])));
                } else if let (Some((project, pad)), Some(slot)) =
                    (pads::pad_from_file_id(fid), json.get("sym").and_then(|v| v.as_u64()))
                {
                    let slot = slot as u16;
                    let text = format!("pad {} {} now plays sound {slot}", GROUPS[pad.group], pads::PAD_LABELS[pad.pad]);
                    let _ = self.events.send(Event::DeviceMessage(text));
                    let _ = self.events.send(Event::DevicePadSound { project, pad, slot });
                }
                self.ctx.request_repaint();
            }
        }
        if let Some(g) = group {
            self.emit(Event::DeviceGroup(g));
        }
    }

    fn read_pads(&mut self) {
        if self.conn.is_none() {
            self.emit(Event::ReadFailed("KO II not connected.".into()));
            return;
        }
        let result = self.read_pads_inner();
        match result {
            Ok(r) => self.emit(Event::ReadDone(r)),
            Err(ReadError::Other(msg)) => self.emit(Event::ReadFailed(msg)),
            Err(ReadError::Debug(text)) => self.emit(Event::ReadFailed(format!(
                "The KO II reported an internal error ({text}). Turn the KO II off and on before trying again."
            ))),
        }
    }

    fn read_pads_inner(&mut self) -> Result<ReadResult, ReadError> {
        let conn = self.conn.as_mut().ok_or_else(|| "KO II not connected.".to_string())?;
        while conn.sysex.try_recv().is_ok() {} // throw away old messages

        let _ = self.events.send(Event::ReadProgress("Saying hello…".into()));
        self.ctx.request_repaint();
        // GREET: "key:value;..." text with os_version. Optional; ignore if it fails.
        let os_version = request(conn, sysex::CMD_GREET, &[])
            .ok()
            .and_then(|body| greet_value(&body, "os_version"));

        // FILE INIT, same bytes the official Sample Tool sends (krate PROTOCOL.md).
        request(conn, sysex::CMD_FILE, &[0x01, 0x01, 0x00, 0x40, 0x00, 0x00])?;

        // Active project: METADATA GET on /projects (2000) -> {"active": project_fid}.
        let projects = metadata_get(conn, 2000)?;
        let active = projects.get("active").and_then(|v| v.as_u64()).unwrap_or(0);
        if !(3000..=11000).contains(&active) || active % 1000 != 0 {
            return Err(format!("Unexpected project number from the KO II ({active}).").into());
        }
        let project = ((active - 2000) / 1000) as u8;

        let _ = self.events.send(Event::ReadProgress("Reading sound list…".into()));
        self.ctx.request_repaint();
        let sound_names = list_names(conn, 1000)?;

        let mut slots = vec![vec![None; 12]; 4];
        for (g, group_slots) in slots.iter_mut().enumerate() {
            let _ = self
                .events
                .send(Event::ReadProgress(format!("Reading group {}…", GROUPS[g])));
            self.ctx.request_repaint();
            for (p, slot) in group_slots.iter_mut().enumerate() {
                let meta = metadata_get(conn, pads::pad_file_id(project, g, p))?;
                *slot = meta
                    .get("sym")
                    .and_then(|v| v.as_u64())
                    .filter(|&s| (1..=999).contains(&s))
                    .map(|s| s as u16);
            }
        }

        // The /sounds list only gives file names like "007.pcm". The real sound name
        // lives in each slot's metadata ("name"), so read it for the slots in use.
        let _ = self.events.send(Event::ReadProgress("Reading sound names…".into()));
        self.ctx.request_repaint();
        let mut used: Vec<u16> = slots.iter().flatten().flatten().copied().collect();
        used.sort_unstable();
        used.dedup();
        let mut names = HashMap::new();
        for slot in used {
            let meta = metadata_get(conn, slot)?;
            let name = meta
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(str::to_string)
                .or_else(|| sound_names.get(&slot).map(|n| n.trim_end_matches(".pcm").to_string()));
            if let Some(name) = name {
                names.insert(slot, name);
            }
        }

        Ok(ReadResult { project, os_version, slots, sound_names: names })
    }
}

fn open(port_name: &str, shared_out: &MidiOut, events: Sender<Event>, ctx: egui::Context) -> Result<Connection, String> {
    const BUSY: &str = "KO II found, but another app is using it. Close the TE Sample Tool web page or other music apps.";
    let out = MidiOutput::new("KO II Buddy").map_err(|e| e.to_string())?;
    let out_port = out
        .ports()
        .into_iter()
        .find(|p| out.port_name(p).ok().as_deref() == Some(port_name))
        .ok_or("KO II port went away.")?;
    let out_conn = out.connect(&out_port, "koii-out").map_err(|_| BUSY.to_string())?;

    let mut input = MidiInput::new("KO II Buddy").map_err(|e| e.to_string())?;
    input.ignore(Ignore::None); // we need SysEx
    let in_port = input
        .ports()
        .into_iter()
        .find(|p| input.port_name(p).ok().is_some_and(|n| is_ko2_port(&n)))
        .ok_or("KO II input port not found.")?;
    let (tx, rx) = mpsc::channel();
    let mut assembler = SysexAssembler::default();
    let in_conn = input
        .connect(
            &in_port,
            "koii-in",
            move |_t, bytes, _| {
                // Clock and "still here" messages come many times a second: skip them.
                if bytes.is_empty() || matches!(bytes[0], 0xF8 | 0xFE) {
                    return;
                }
                if bytes[0] != 0xF0 && bytes[0] >= 0x80 && bytes[0] < 0xF0 {
                    let text = describe(bytes);
                    midi_log::write(&text);
                    let _ = events.send(Event::DeviceMessage(text));
                    ctx.request_repaint();
                } else if bytes[0] >= 0xF8 {
                    let text = match bytes[0] {
                        0xFA => "start",
                        0xFB => "continue",
                        0xFC => "stop",
                        _ => "realtime message",
                    };
                    midi_log::write(text);
                    let _ = events.send(Event::DeviceMessage(text.to_string()));
                    ctx.request_repaint();
                    return;
                }
                // Pads pressed on the KO II come in as normal note on / note off messages.
                if let [status, note, velocity, ..] = *bytes {
                    let on = match status & 0xF0 {
                        0x90 => Some(velocity > 0),
                        0x80 => Some(false),
                        _ => None,
                    };
                    if let Some(on) = on {
                        let _ = events.send(Event::DeviceNote { note, on });
                        ctx.request_repaint();
                        return;
                    }
                }
                if let Some(frame) = assembler.push(bytes) {
                    let _ = tx.send(frame);
                }
            },
            (),
        )
        .map_err(|_| BUSY.to_string())?;

    shared_out.set(Some(out_conn));
    Ok(Connection {
        port_name: port_name.to_string(),
        out: shared_out.clone(),
        _input: in_conn,
        sysex: rx,
        next_id: 1,
    })
}

/// Windows delivers long SysEx in 1024-byte pieces; glue them back together.
#[derive(Default)]
struct SysexAssembler {
    buf: Vec<u8>,
}

impl SysexAssembler {
    fn push(&mut self, bytes: &[u8]) -> Option<Vec<u8>> {
        match bytes.first() {
            Some(0xF0) => self.buf = bytes.to_vec(),
            Some(b) if *b < 0x80 || *b == 0xF7 => {
                if self.buf.is_empty() {
                    return None; // a normal MIDI message or a stray piece
                }
                self.buf.extend_from_slice(bytes);
            }
            _ => return None, // notes, clock, etc.
        }
        if self.buf.last() == Some(&0xF7) {
            Some(std::mem::take(&mut self.buf))
        } else {
            None
        }
    }
}

/// Send one request and wait for its answer (3 s).
fn request(conn: &mut Connection, command: u8, raw_body: &[u8]) -> Result<Vec<u8>, ReadError> {
    let id = conn.next_id & 0x0FFF;
    conn.next_id = conn.next_id.wrapping_add(1) & 0x0FFF;
    if conn.next_id == 0 {
        conn.next_id = 1;
    }
    if !conn.out.send(&sysex::build_request(id, command, raw_body)) {
        return Err("Could not send to the KO II.".to_string().into());
    }

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let frame = match conn.sysex.recv_timeout(left) {
            Ok(f) => f,
            Err(_) => return Err("The KO II did not answer. Check the cable and try again.".to_string().into()),
        };
        match sysex::parse_reply(&frame, id, command) {
            Reply::Answer { status: 0, body } => return Ok(body),
            Reply::Answer { body, .. } => {
                let reason = String::from_utf8_lossy(&body).trim_matches(['\0', ' ']).to_string();
                return Err(format!("The KO II said no: {reason}").into());
            }
            Reply::Debug(text) => return Err(ReadError::Debug(text)),
            Reply::Other => continue,
        }
    }
}

/// METADATA GET (07 02 fid page), all pages, parsed as JSON.
fn metadata_get(conn: &mut Connection, fid: u16) -> Result<serde_json::Map<String, serde_json::Value>, ReadError> {
    let mut text = Vec::new();
    for page in 0u16..32 {
        let [f_hi, f_lo] = fid.to_be_bytes();
        let [p_hi, p_lo] = page.to_be_bytes();
        let body = request(conn, sysex::CMD_FILE, &[0x07, 0x02, f_hi, f_lo, p_hi, p_lo])?;
        if body.len() <= 2 {
            break;
        }
        let chunk = &body[2..];
        let end = chunk.iter().position(|&b| b == 0).unwrap_or(chunk.len());
        text.extend_from_slice(&chunk[..end]);
        if body.last() == Some(&0) {
            break;
        }
    }
    if text.is_empty() {
        return Ok(Default::default());
    }
    match serde_json::from_slice::<serde_json::Value>(&text) {
        Ok(serde_json::Value::Object(map)) => Ok(map),
        _ => Ok(Default::default()),
    }
}

/// LIST (04 page node), all pages: child id -> name.
fn list_names(conn: &mut Connection, node: u16) -> Result<HashMap<u16, String>, ReadError> {
    let mut names = HashMap::new();
    for page in 0u16..200 {
        let [n_hi, n_lo] = node.to_be_bytes();
        let [p_hi, p_lo] = page.to_be_bytes();
        let body = request(conn, sysex::CMD_FILE, &[0x04, p_hi, p_lo, n_hi, n_lo])?;
        if body.len() <= 2 {
            break;
        }
        parse_list_page(&body[2..], &mut names);
    }
    Ok(names)
}

/// Entries: <id u16> <flags u8> <size u32> <name> 00
fn parse_list_page(mut rest: &[u8], names: &mut HashMap<u16, String>) {
    while rest.len() >= 7 {
        let id = u16::from_be_bytes([rest[0], rest[1]]);
        let after = &rest[7..];
        let end = after.iter().position(|&b| b == 0).unwrap_or(after.len());
        let name = String::from_utf8_lossy(&after[..end]).trim().to_string();
        names.insert(id, name);
        rest = after.get(end + 1..).unwrap_or(&[]);
    }
}

/// GREET reply is text like "product:EP-133;os_version:2.5.1;...".
fn greet_value(body: &[u8], key: &str) -> Option<String> {
    let text = String::from_utf8_lossy(body);
    text.trim_matches('\0')
        .split(';')
        .filter_map(|pair| pair.split_once(':'))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembler_glues_split_sysex() {
        let mut a = SysexAssembler::default();
        assert_eq!(a.push(&[0x90, 36, 100]), None);
        assert_eq!(a.push(&[0xF0, 0x00, 0x20]), None);
        assert_eq!(a.push(&[0x76, 0x33, 0xF7]), Some(vec![0xF0, 0x00, 0x20, 0x76, 0x33, 0xF7]));
        assert_eq!(a.push(&[0xF0, 0x01, 0xF7]), Some(vec![0xF0, 0x01, 0xF7]));
    }

    #[test]
    fn list_page_parsing() {
        let mut page = vec![0x00, 0x01, 0x02, 0, 0, 0x10, 0x00];
        page.extend(b"kick\0");
        page.extend([0x00, 0x64, 0x00, 0, 0, 0x00, 0x20]);
        page.extend(b"snare\0");
        let mut names = HashMap::new();
        parse_list_page(&page, &mut names);
        assert_eq!(names.get(&1).map(String::as_str), Some("kick"));
        assert_eq!(names.get(&100).map(String::as_str), Some("snare"));
    }

    #[test]
    fn greet_parsing() {
        let body = b"product:EP-133;sku:TE032AS001;os_version:2.5.1;serial:x\0";
        assert_eq!(greet_value(body, "os_version").as_deref(), Some("2.5.1"));
        assert_eq!(greet_value(body, "missing"), None);
    }

    #[test]
    fn port_names() {
        assert!(is_ko2_port("EP-133"));
        assert!(is_ko2_port("2- EP-133"));
        assert!(!is_ko2_port("Microsoft GS Wavetable Synth"));
    }
}
