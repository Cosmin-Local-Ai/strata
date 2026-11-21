//! Now Playing: Windows' system media sessions (GlobalSystemMediaTransportControlsSessionManager),
//! which every app that reports to Windows' media overlay uses (Spotify, browsers, Windows media apps).
//! Event-driven: Windows notifies us of session, track and playback changes; no polling.

use serde::Serialize;
use std::sync::{Arc, Mutex};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::{DataReader, InputStreamOptions};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub active: bool,
    /// App user model ID of the source app (e.g. Spotify.exe), for its icon.
    pub app: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub can_prev: bool,
    pub can_next: bool,
    pub can_play_pause: bool,
    /// Seconds.
    pub position: f64,
    pub duration: f64,
    /// Unix ms when `position` was sampled (the UI extrapolates while playing).
    pub sampled_at: u64,
    /// data:image/...;base64 cover art, or empty (UI falls back to the app icon).
    pub art: String,
}

type Emit = Arc<dyn Fn(&MediaState) + Send + Sync>;

struct Inner {
    manager: Option<Manager>,
    session: Option<Session>,
    tokens: Vec<i64>,
    last: MediaState,
    art_key: String,
    art: String,
}

#[derive(Clone)]
pub struct Media(Arc<Mutex<Inner>>, Emit);

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn b64_pub(data: &[u8]) -> String {
    b64(data)
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn read_thumbnail(s: &Session) -> Option<String> {
    let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let thumb = props.Thumbnail().ok()?;
    let stream = thumb.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 8 << 20 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.SetInputStreamOptions(InputStreamOptions::ReadAhead).ok()?;
    let n = reader.LoadAsync(size).ok()?.join().ok()?;
    let mut buf = vec![0u8; n as usize];
    reader.ReadBytes(&mut buf).ok()?;
    let mime = if buf.starts_with(&[0x89, b'P']) { "image/png" } else { "image/jpeg" };
    Some(format!("data:{mime};base64,{}", b64(&buf)))
}

impl Media {
    pub fn start(emit: impl Fn(&MediaState) + Send + Sync + 'static) -> Option<Media> {
        let manager = Manager::RequestAsync().ok()?.join().ok()?;
        let m = Media(Arc::new(Mutex::new(Inner { manager: Some(manager.clone()), session: None, tokens: vec![], last: MediaState::default(), art_key: String::new(), art: String::new() })), Arc::new(emit));
        let me = m.clone();
        let _ = manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            me.attach();
            Ok(())
        }));
        m.attach();
        Some(m)
    }

    /// Follow the current session (the one Windows' media overlay shows).
    fn attach(&self) {
        let session = {
            let g = self.0.lock().unwrap();
            g.manager.as_ref().and_then(|mg| mg.GetCurrentSession().ok())
        };
        {
            let mut g = self.0.lock().unwrap();
            if let Some(old) = g.session.take() {
                for (i, t) in g.tokens.drain(..).enumerate() {
                    let _ = match i {
                        0 => old.RemoveMediaPropertiesChanged(t),
                        1 => old.RemovePlaybackInfoChanged(t),
                        _ => old.RemoveTimelinePropertiesChanged(t),
                    };
                }
            }
            g.session = session.clone();
        }
        if let Some(s) = session {
            let (a, b, c) = (self.clone(), self.clone(), self.clone());
            let t1 = s.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| { a.refresh(true); Ok(()) })).unwrap_or(0);
            let t2 = s.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| { b.refresh(false); Ok(()) })).unwrap_or(0);
            let t3 = s.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| { c.refresh(false); Ok(()) })).unwrap_or(0);
            self.0.lock().unwrap().tokens = vec![t1, t2, t3];
        }
        self.refresh(true);
    }

    fn refresh(&self, track_changed: bool) {
        let session = self.0.lock().unwrap().session.clone();
        let mut st = MediaState { sampled_at: now_ms(), ..Default::default() };
        if let Some(s) = &session {
            st.active = true;
            st.app = s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default();
            if let Ok(p) = s.TryGetMediaPropertiesAsync().and_then(|op| op.join()) {
                st.title = p.Title().map(|h| h.to_string()).unwrap_or_default();
                st.artist = p.Artist().map(|h| h.to_string()).unwrap_or_default();
                st.album = p.AlbumTitle().map(|h| h.to_string()).unwrap_or_default();
            }
            if let Ok(pi) = s.GetPlaybackInfo() {
                st.playing = pi.PlaybackStatus().map(|x| x == Status::Playing).unwrap_or(false);
                if let Ok(c) = pi.Controls() {
                    st.can_prev = c.IsPreviousEnabled().unwrap_or(false);
                    st.can_next = c.IsNextEnabled().unwrap_or(false);
                    st.can_play_pause = c.IsPlayPauseToggleEnabled().unwrap_or(false);
                }
            }
            if let Ok(t) = s.GetTimelineProperties() {
                let secs = |ts: windows::Foundation::TimeSpan| ts.Duration as f64 / 10_000_000.0;
                st.position = t.Position().map(secs).unwrap_or(0.0);
                st.duration = (t.EndTime().map(secs).unwrap_or(0.0) - t.StartTime().map(secs).unwrap_or(0.0)).max(0.0);
            }
            // Cover art: re-read only when the track changes (it's the expensive part).
            let key = format!("{}|{}|{}", st.app, st.title, st.artist);
            let need_art = track_changed || self.0.lock().unwrap().art_key != key;
            if need_art {
                let art = read_thumbnail(s).unwrap_or_default();
                let mut g = self.0.lock().unwrap();
                g.art_key = key;
                g.art = art;
            }
            st.art = self.0.lock().unwrap().art.clone();
        }
        let changed = {
            let mut g = self.0.lock().unwrap();
            // Position-only changes while paused don't need to reach the UI.
            let mut cmp = st.clone();
            cmp.sampled_at = g.last.sampled_at;
            if !st.playing { cmp.position = g.last.position; }
            let changed = cmp != g.last;
            g.last = st.clone();
            changed
        };
        if changed {
            (self.1)(&st);
        }
    }

    /// Short name of the playing app for matching its audio session ("spotify" from "Spotify.exe" or a Store AUMID).
    pub fn app_hint(&self) -> String {
        let app = self.0.lock().unwrap().last.app.to_lowercase();
        let base = app.rsplit('!').last().unwrap_or(&app).split(['.', '_']).next().unwrap_or("").to_string();
        if base == "spotifyab" { "spotify".into() } else { base }
    }

    pub fn playing(&self) -> bool {
        self.0.lock().unwrap().last.playing
    }

    pub fn state(&self) -> MediaState {
        self.0.lock().unwrap().last.clone()
    }

    pub fn control(&self, action: &str) -> Result<(), String> {
        let s = self.0.lock().unwrap().session.clone().ok_or("nothing is playing")?;
        let r = match action {
            "toggle" | "playpause" => s.TryTogglePlayPauseAsync(),
            "next" => s.TrySkipNextAsync(),
            "prev" => s.TrySkipPreviousAsync(),
            "play" => s.TryPlayAsync(),
            "pause" => s.TryPauseAsync(),
            _ => return Err(format!("unknown media action {action}")),
        };
        r.and_then(|op| op.join()).map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        let s = self.0.lock().unwrap().session.clone().ok_or("nothing is playing")?;
        s.TryChangePlaybackPositionAsync((seconds * 10_000_000.0) as i64).and_then(|op| op.join()).map(|_| ()).map_err(|e| e.to_string())
    }
}
