//! Settings chosen from the menu bar, kept in `~/Library/Application Support/Ligarview/config`
//! as simple `key=value` lines.

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub portrait: bool,
    pub fps: u32,
    pub bitrate: u32,
    pub hevc: bool,
    /// Whether tablets may connect at all (Wi-Fi and USB). Switched from the window.
    pub sharing: bool,
    /// Code of the paired tablet, which it must send over Wi-Fi; 0 while nothing is paired.
    pub code: u32,
    /// Name of the paired tablet, for display.
    pub tablet: String,
    /// Whether the tablet's pen may move the pointer and click on the Mac.
    pub control: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { portrait: false, fps: 60, bitrate: 30, hevc: false, sharing: true, code: 0, tablet: String::new(), control: false }
    }
}

fn path() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Ligarview/config"))
}

impl Config {
    pub fn load() -> Self {
        let mut c = Self::default();
        let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else { return c };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match (k.trim(), v.trim()) {
                ("orientation", v) => c.portrait = v == "portrait",
                ("fps", v) => c.fps = v.parse().unwrap_or(c.fps),
                ("bitrate", v) => c.bitrate = v.parse().unwrap_or(c.bitrate),
                ("codec", v) => c.hevc = v == "hevc",
                ("sharing", v) => c.sharing = v != "off",
                ("code", v) => c.code = v.parse().unwrap_or(0),
                ("tablet", v) => c.tablet = v.to_string(),
                ("control", v) => c.control = v == "on",
                _ => {}
            }
        }
        c
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let text = format!(
            "orientation={}\nfps={}\nbitrate={}\ncodec={}\nsharing={}\ncode={}\ntablet={}\ncontrol={}\n",
            if self.portrait { "portrait" } else { "landscape" },
            self.fps,
            self.bitrate,
            if self.hevc { "hevc" } else { "h264" },
            if self.sharing { "on" } else { "off" },
            self.code,
            self.tablet.replace('\n', " "),
            if self.control { "on" } else { "off" }
        );
        let _ = std::fs::write(p, text);
    }
}
