//! Local, paragraph-scoped conversations. Agent responses are accepted only
//! for the latest round on the exact captured document revision and text.
use crate::{EditError, Workbench};
use serde::{Deserialize, Serialize};
use std::ops::Range;

const MAX_THREADS: usize = 128;
const MAX_MESSAGES: usize = 128;
const MAX_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speaker {
    User,
    Agent,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub speaker: Speaker,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Supplied by the trusted host; the core never reads a clock or identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommentThread {
    pub(crate) range: Range<usize>,
    pub(crate) revision: u64,
    pub(crate) original: String,
    pub(crate) round: u64,
    pub(crate) messages: Vec<Message>,
    pub(crate) resolved: bool,
    /// The comment was sent with 「问 AI」: the agent processes it and its
    /// follow-up replies automatically. Defaults to false for threads
    /// persisted before this flag existed.
    #[serde(default)]
    pub(crate) ask_ai: bool,
}
impl CommentThread {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn original(&self) -> &str {
        &self.original
    }
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
    pub fn resolved(&self) -> bool {
        self.resolved
    }
    pub fn ask_ai(&self) -> bool {
        self.ask_ai
    }
    pub fn transcript(&self) -> String {
        self.messages
            .iter()
            .map(|m| {
                format!(
                    "{}\n{}",
                    if m.speaker == Speaker::User {
                        "你"
                    } else {
                        "Agent"
                    },
                    m.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentRequest {
    pub thread: usize,
    pub round: u64,
    pub revision: u64,
    pub original: String,
    pub messages: Vec<Message>,
}
impl Workbench {
    /// Stamp a newly appended message once. No document/round/revision change
    /// or writing authority; persisted legacy messages remain unknown.
    pub fn stamp_last_message(
        &mut self,
        id: usize,
        author: &str,
        created_at: i64,
    ) -> Result<(), EditError> {
        if author.trim().is_empty() || author.len() > 128 || created_at < 0 {
            return Err(EditError::InvalidSelection);
        }
        if self.state_bytes() + author.len() > crate::MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        let message = self
            .threads
            .get_mut(id)
            .and_then(|t| t.messages.last_mut())
            .ok_or(EditError::UnknownAnnotation)?;
        if message.author.is_some() || message.created_at.is_some() {
            return Err(EditError::Conflict);
        }
        message.author = Some(author.trim().to_owned());
        message.created_at = Some(created_at);
        Ok(())
    }
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
    pub fn thread(&self, id: usize) -> Option<&CommentThread> {
        self.threads.get(id)
    }
    pub fn comment(&mut self, range: Range<usize>, text: &str) -> Result<usize, EditError> {
        self.comment_with_ai(range, text, false)
    }
    pub fn comment_with_ai(
        &mut self,
        range: Range<usize>,
        text: &str,
        ask_ai: bool,
    ) -> Result<usize, EditError> {
        check_message(text)?;
        if self.threads.len() >= MAX_THREADS {
            return Err(EditError::StateLimit);
        }
        if range.start >= range.end {
            return Err(EditError::InvalidSelection);
        }
        let original = self
            .text
            .get(range.clone())
            .ok_or(EditError::InvalidSelection)?
            .to_owned();
        if self.state_bytes() + original.len() + text.len() > crate::MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        let id = self.threads.len();
        self.threads.push(CommentThread {
            range,
            original,
            revision: self.revision,
            round: 1,
            messages: vec![Message {
                speaker: Speaker::User,
                text: text.trim().to_owned(),
                author: None,
                created_at: None,
            }],
            resolved: false,
            ask_ai,
        });
        Ok(id)
    }
    pub fn reply(&mut self, id: usize, text: &str) -> Result<(), EditError> {
        check_message(text)?;
        if self.state_bytes() + text.len() > crate::MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        let thread = self
            .threads
            .get_mut(id)
            .ok_or(EditError::UnknownAnnotation)?;
        if thread.resolved || thread.messages.len() >= MAX_MESSAGES {
            return Err(EditError::StateLimit);
        }
        let round = thread
            .round
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        thread.messages.push(Message {
            speaker: Speaker::User,
            text: text.trim().to_owned(),
            author: None,
            created_at: None,
        });
        thread.round = round;
        Ok(())
    }
    /// Explicit human rebind after an overlapping edit/undo. Old inflight
    /// requests cannot match the new round/revision even if the text is equal.
    pub fn rebind_thread(&mut self, id: usize, range: Range<usize>) -> Result<(), EditError> {
        if range.start >= range.end {
            return Err(EditError::InvalidSelection);
        }
        let text = self
            .text
            .get(range.clone())
            .ok_or(EditError::InvalidSelection)?
            .to_owned();
        let thread = self.threads.get(id).ok_or(EditError::UnknownAnnotation)?;
        if self.state_bytes() - thread.original.len() + text.len() > crate::MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        let round = thread
            .round
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        let thread = &mut self.threads[id];
        thread.range = range;
        thread.original = text;
        thread.revision = self.revision;
        thread.round = round;
        thread.resolved = false;
        Ok(())
    }
    pub fn reopen_thread(&mut self, id: usize) -> Result<(), EditError> {
        let thread = self
            .threads
            .get_mut(id)
            .ok_or(EditError::UnknownAnnotation)?;
        thread.round = thread
            .round
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        thread.resolved = false;
        Ok(())
    }
    /// Move only anchors whose original is provably outside the changed span.
    /// Touching an anchor invalidates it; equal text elsewhere is never searched.
    pub(crate) fn relocate_threads(&mut self, next: &str, next_revision: u64) {
        let mut prefix = self
            .text
            .bytes()
            .zip(next.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !self.text.is_char_boundary(prefix) || !next.is_char_boundary(prefix) {
            prefix -= 1;
        }
        let mut suffix = self.text[prefix..]
            .bytes()
            .rev()
            .zip(next[prefix..].bytes().rev())
            .take_while(|(a, b)| a == b)
            .count();
        while !self.text.is_char_boundary(self.text.len() - suffix)
            || !next.is_char_boundary(next.len() - suffix)
        {
            suffix -= 1;
        }
        let old_end = self.text.len() - suffix;
        for thread in &mut self.threads {
            if thread.revision != self.revision
                || self.text.get(thread.range.clone()) != Some(thread.original.as_str())
            {
                continue;
            }
            let range = if thread.range.end <= prefix {
                Some(thread.range.clone())
            } else if thread.range.start >= old_end {
                let start = next.len() - suffix + (thread.range.start - old_end);
                Some(start..start + thread.original.len())
            } else {
                None
            };
            if let Some(range) = range
                && next.get(range.clone()) == Some(thread.original.as_str())
            {
                thread.range = range;
                thread.revision = next_revision;
            }
        }
    }
    pub fn resolve_thread(&mut self, id: usize) -> Result<(), EditError> {
        let thread = self
            .threads
            .get_mut(id)
            .ok_or(EditError::UnknownAnnotation)?;
        thread.round = thread
            .round
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        thread.resolved = true;
        Ok(())
    }
    pub fn comment_request(&self, id: usize) -> Result<CommentRequest, EditError> {
        let thread = self.threads.get(id).ok_or(EditError::UnknownAnnotation)?;
        if thread.resolved {
            return Err(EditError::ProposalAlreadyResolved);
        }
        if thread.revision != self.revision
            || self.text.get(thread.range.clone()) != Some(thread.original.as_str())
        {
            return Err(EditError::Conflict);
        }
        Ok(CommentRequest {
            thread: id,
            round: thread.round,
            revision: thread.revision,
            original: thread.original.clone(),
            messages: thread.messages.clone(),
        })
    }
    /// UI must explicitly enable automatic editing for this request. No approval
    /// survives restart. A successful edit moves this thread to its new text.
    pub fn apply_comment_result(
        &mut self,
        request: &CommentRequest,
        replacement: &str,
        explanation: &str,
    ) -> Result<(), EditError> {
        check_message(explanation)?;
        Self::check_size(replacement.len())?;
        let fresh = self.comment_request(request.thread)?;
        if fresh != *request {
            return Err(EditError::Conflict);
        }
        let thread = &self.threads[request.thread];
        if thread.messages.len() >= MAX_MESSAGES {
            return Err(EditError::StateLimit);
        }
        let end = thread.range.start + replacement.len();
        let range = thread.range.clone();
        let mut next = self.text.clone();
        next.replace_range(range.clone(), replacement);
        Self::check_size(next.len())?;
        // Bound the added transcript/anchor before making any document change.
        if self.state_bytes() + explanation.len() + replacement.len() > crate::MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        let mut candidate = self.clone();
        candidate.commit(next)?;
        let thread = &mut candidate.threads[request.thread];
        thread.range = range.start..end;
        thread.original = replacement.to_owned();
        thread.revision = candidate.revision;
        thread.messages.push(Message {
            speaker: Speaker::Agent,
            text: explanation.trim().to_owned(),
            author: None,
            created_at: None,
        });
        if candidate.state_bytes() > crate::MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        *self = candidate;
        Ok(())
    }
}
fn check_message(text: &str) -> Result<(), EditError> {
    if text.trim().is_empty() {
        return Err(EditError::EmptyInstruction);
    }
    if text.len() > MAX_MESSAGE_BYTES {
        return Err(EditError::StateLimit);
    }
    Ok(())
}
pub(crate) fn validate_threads(
    threads: &[CommentThread],
    text: &str,
    revision: u64,
) -> Result<(), EditError> {
    if threads.len() > MAX_THREADS {
        return Err(EditError::InvalidSnapshot);
    }
    for t in threads {
        if t.range.start > t.range.end
            || t.range.end > crate::MAX_DOCUMENT_BYTES
            || t.revision > revision
            || t.original.len() != t.range.end - t.range.start
            || t.round == 0
            || t.messages.is_empty()
            || t.messages.len() > MAX_MESSAGES
            || t.messages[0].speaker != Speaker::User
        {
            return Err(EditError::InvalidSnapshot);
        }
        if t.revision == revision && text.get(t.range.clone()) != Some(t.original.as_str()) {
            return Err(EditError::InvalidSnapshot);
        }
        for m in &t.messages {
            check_message(&m.text)?;
            if m.author
                .as_ref()
                .is_some_and(|a| a.trim().is_empty() || a.len() > 128)
                || m.created_at.is_some_and(|t| t < 0)
            {
                return Err(EditError::InvalidSnapshot);
            }
        }
    }
    Ok(())
}
