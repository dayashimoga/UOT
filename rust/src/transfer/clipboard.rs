//! Clipboard Transfer Support
//!
//! Enables sending/receiving text, URLs, and small data via clipboard.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Clipboard content types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardContentType {
    PlainText,
    Url,
    Html,
    Image,
}

impl std::fmt::Display for ClipboardContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlainText => write!(f, "text/plain"),
            Self::Url => write!(f, "text/uri-list"),
            Self::Html => write!(f, "text/html"),
            Self::Image => write!(f, "image/png"),
        }
    }
}

/// Clipboard transfer item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: Uuid,
    pub content_type: ClipboardContentType,
    pub data: String,
    pub preview: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ClipboardItem {
    /// Create a new text clipboard item.
    pub fn text(data: String) -> Self {
        let preview = if data.len() > 100 {
            Some(format!("{}…", &data[..100]))
        } else {
            Some(data.clone())
        };
        Self {
            id: Uuid::new_v4(),
            content_type: ClipboardContentType::PlainText,
            data,
            preview,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a URL clipboard item.
    pub fn url(url: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            content_type: ClipboardContentType::Url,
            preview: Some(url.clone()),
            data: url,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Detect content type from string.
    pub fn auto_detect(data: String) -> Self {
        if data.starts_with("http://") || data.starts_with("https://") {
            Self::url(data)
        } else if data.starts_with("<!") || data.starts_with("<html") {
            Self {
                id: Uuid::new_v4(),
                content_type: ClipboardContentType::Html,
                preview: Some(if data.len() > 100 {
                    format!("{}…", &data[..100])
                } else {
                    data.clone()
                }),
                data,
                timestamp: chrono::Utc::now(),
            }
        } else {
            Self::text(data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_item_text() {
        let item = ClipboardItem::text("Hello Clipboard".to_string());
        assert_eq!(item.content_type.to_string(), "text/plain");
        assert_eq!(item.data, "Hello Clipboard");
        assert_eq!(item.preview, Some("Hello Clipboard".to_string()));
    }

    #[test]
    fn test_clipboard_item_auto_detect_url() {
        let item = ClipboardItem::auto_detect("https://uot.app".to_string());
        assert_eq!(item.content_type.to_string(), "text/uri-list");
        assert_eq!(item.data, "https://uot.app");
    }

    #[test]
    fn test_clipboard_item_auto_detect_html() {
        let item = ClipboardItem::auto_detect("<html><body>Hi</body></html>".to_string());
        assert_eq!(item.content_type.to_string(), "text/html");
    }

    #[test]
    fn test_clipboard_item_edge_cases() {
        let long_text = "a".repeat(150);
        let item = ClipboardItem::text(long_text);
        assert!(item.preview.unwrap().ends_with('…'));

        let img_type = ClipboardContentType::Image;
        assert_eq!(img_type.to_string(), "image/png");
    }
}
