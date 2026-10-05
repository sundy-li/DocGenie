//! Host clock / presentation only; document-core never reads environment/time.
use document_core::{Message, Speaker};
pub fn timestamp() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp().max(0)
}
pub fn author(message: &Message) -> &str {
    message
        .author
        .as_deref()
        .unwrap_or(if message.speaker == Speaker::User {
            "你"
        } else {
            "Agent"
        })
}
pub fn label(message: &Message) -> String {
    let Some(seconds) = message.created_at else {
        return "时间未知".into();
    };
    let Ok(date) = time::OffsetDateTime::from_unix_timestamp(seconds) else {
        return "时间未知".into();
    };
    let offset = time::UtcOffset::local_offset_at(date).unwrap_or(time::UtcOffset::UTC);
    let local = date.to_offset(offset);
    format!(
        "{}月{}日 {:02}:{:02}",
        local.month() as u8,
        local.day(),
        local.hour(),
        local.minute()
    )
}
pub fn excerpt(text: &str, limit: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let mut result: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_time_stays_unknown_and_new_time_is_displayed() {
        let mut m = Message {
            speaker: Speaker::User,
            text: "评论".into(),
            author: None,
            created_at: None,
        };
        assert_eq!(author(&m), "你");
        assert_eq!(label(&m), "时间未知");
        m.created_at = Some(1791130000);
        m.author = Some("本地作者".into());
        assert_eq!(author(&m), "本地作者");
        assert!(label(&m).contains('月'));
        assert_eq!(excerpt("中文\n 长段落", 3), "中文 …");
    }
}
