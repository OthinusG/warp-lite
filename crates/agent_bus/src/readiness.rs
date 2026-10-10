//! Lifecycle evidence and draft protection are independent delivery gates.
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Starting,
    Idle,
    Working,
    WaitingApproval,
    WaitingInput,
    Cancelled,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_unicode_edits_preserve_drafts_and_unknown_edits_fail_closed() {
        for text in ["abc", "几个字", "e\u{301}", "👨‍👩‍👧‍👦"] {
            let mut draft = Draft::default();
            assert_eq!(draft.input(text.as_bytes()), None);
            assert_eq!(draft.state(), "present");
            let deletes = vec![0x7f; text.graphemes(true).count()];
            assert_eq!(draft.input(&deletes), None);
            assert!(draft.is_empty());
        }
        let mut draft = Draft::default();
        assert_eq!(draft.input(b"\x1b[200~first\nsecond\x1b[201~"), None);
        assert_eq!(draft.state(), "present", "multiline paste is not a submit");
        assert_eq!(draft.input(b"\r"), Some(Activity::Working));
        assert!(draft.is_empty());
        assert_eq!(draft.input(b"\x1b[A"), None);
        assert_eq!(
            draft.state(),
            "unknown",
            "history can restore text invisible to the broker"
        );
        draft.input(b"\x7f\x7f");
        assert_eq!(draft.state(), "unknown");
        assert_eq!(draft.input(b"\x03"), Some(Activity::Cancelled));
    }
}

impl Activity {
    pub fn is_idle(self) -> bool {
        self == Self::Idle
    }
}

/// A bounded, memory-only native draft; never serialized or included in diagnostics.
#[derive(Default)]
pub struct Draft {
    text: String,
    unknown: bool,
}

impl Draft {
    pub fn state(&self) -> &'static str {
        if self.unknown {
            "unknown"
        } else if !self.text.is_empty() {
            "present"
        } else {
            "empty"
        }
    }

    pub fn is_empty(&self) -> bool {
        self.state() == "empty"
    }

    pub fn invalidate(&mut self) {
        self.unknown = true;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Only append/backspace at the known end of the prompt can prove it empty.
    /// ponytail: history/cursor/clipboard edits remain unknown; use native editor snapshots when vendors expose them.
    pub fn input(&mut self, bytes: &[u8]) -> Option<Activity> {
        if let Some(paste) = bytes
            .strip_prefix(b"\x1b[200~")
            .and_then(|bytes| bytes.strip_suffix(b"\x1b[201~"))
        {
            if let Ok(text) = std::str::from_utf8(paste) {
                self.append(text);
            } else {
                self.invalidate();
            }
            return None;
        }
        if bytes.starts_with(b"\x1b") && bytes.len() > 1 {
            self.invalidate();
            return None;
        }
        let Ok(text) = std::str::from_utf8(bytes) else {
            self.invalidate();
            return None;
        };
        for character in text.chars() {
            match character {
                '\r' | '\n' => {
                    let submitted = !self.is_empty();
                    self.clear();
                    return submitted.then_some(Activity::Working);
                }
                '\u{3}' => {
                    self.invalidate();
                    return Some(Activity::Cancelled);
                }
                '\u{8}' | '\u{7f}' => {
                    if let Some((index, _)) = self.text.grapheme_indices(true).next_back() {
                        self.text.truncate(index);
                    }
                }
                // These can change menus, selection, history, attachments or cursor position.
                character if character.is_control() => self.invalidate(),
                character => self.append(character.encode_utf8(&mut [0; 4])),
            }
        }
        None
    }

    fn append(&mut self, text: &str) {
        if !self.unknown && self.text.len() + text.len() <= crate::MAX_TEXT {
            self.text.push_str(text);
        } else {
            self.invalidate();
        }
    }
}
