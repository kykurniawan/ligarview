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
    /// Whether the tablet's fingers may scroll the Mac.
    pub finger: bool,
    /// How the tablet connects: over Wi-Fi (pairing code) or over the USB cable. Only that
    /// transport is started; the other stays off.
    pub wifi: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { portrait: false, fps: 60, bitrate: 30, hevc: false, sharing: true, code: 0, tablet: String::new(), control: false, finger: false, wifi: false }
    }
}

fn path() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Ligarview/config"))
}

impl Config {
    pub fn load() -> Self {
        match path().and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(text) => Self::parse(&text),
            None => Self::default(),
        }
    }

    fn parse(text: &str) -> Self {
        let mut c = Self::default();
        let mut chose_connection = false;
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
                ("finger", v) => c.finger = v == "on",
                ("connection", v) => {
                    c.wifi = v == "wifi";
                    chose_connection = true;
                }
                _ => {}
            }
        }
        // Settings saved before this option existed: a tablet that was paired keeps using Wi-Fi.
        if !chose_connection {
            c.wifi = c.code != 0;
        }
        c
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let text = format!(
            "orientation={}\nfps={}\nbitrate={}\ncodec={}\nsharing={}\ncode={}\ntablet={}\ncontrol={}\nfinger={}\nconnection={}\n",
            if self.portrait { "portrait" } else { "landscape" },
            self.fps,
            self.bitrate,
            if self.hevc { "hevc" } else { "h264" },
            if self.sharing { "on" } else { "off" },
            self.code,
            self.tablet.replace('\n', " "),
            if self.control { "on" } else { "off" },
            if self.finger { "on" } else { "off" },
            if self.wifi { "wifi" } else { "usb" }
        );
        let _ = std::fs::write(p, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_installs_use_usb() {
        assert!(!Config::parse("").wifi);
        assert!(!Config::parse("fps=60\n").wifi);
    }

    #[test]
    fn a_paired_tablet_from_before_keeps_wifi() {
        assert!(Config::parse("code=123456\ntablet=Tab\n").wifi);
    }

    #[test]
    fn an_explicit_choice_wins() {
        assert!(!Config::parse("code=123456\nconnection=usb\n").wifi);
        assert!(Config::parse("connection=wifi\n").wifi);
    }

    #[test]
    fn round_trips() {
        let mut c = Config::default();
        c.wifi = true;
        c.code = 42;
        let text = format!("connection={}\ncode={}\n", if c.wifi { "wifi" } else { "usb" }, c.code);
        assert_eq!(Config::parse(&text).wifi, true);
    }
}
