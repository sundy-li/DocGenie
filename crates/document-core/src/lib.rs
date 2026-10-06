//! UI-independent, in-memory document editing and review authority.
//! Byte offsets are UTF-8 boundaries, matching Makepad's TextInput selection.
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Range};
pub mod comments;
pub mod review;
pub mod table;
pub use comments::{CommentChange, CommentRequest, CommentThread, Message, Speaker, TaskMode};

pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
const MAX_HISTORY: usize = 100;
const MAX_RECORDS: usize = 256;
const MAX_STATE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    Applied,
    Rejected,
    Conflicted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidSelection,
    EmptyInstruction,
    DocumentTooLarge,
    UnknownAnnotation,
    UnknownProposal,
    ProposalAlreadyResolved,
    Conflict,
    NothingToUndo,
    RevisionExhausted,
    InvalidSnapshot,
    StateLimit,
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidSelection => "请选择非空文本，选区必须位于 UTF-8 字符边界",
            Self::EmptyInstruction => "请填写修改要求",
            Self::DocumentTooLarge => "文档超过 1 MiB 限制",
            Self::UnknownAnnotation => "批注不存在",
            Self::UnknownProposal => "提案不存在",
            Self::ProposalAlreadyResolved => "提案已经处理，不能重复执行",
            Self::Conflict => "原文版本已变化，请重新选择并生成提案",
            Self::NothingToUndo => "没有可撤销的修改",
            Self::RevisionExhausted => "版本编号已耗尽",
            Self::InvalidSnapshot => "项目中的文档状态无效，未恢复",
            Self::StateLimit => "批注、提案或历史超过工作区限额",
        })
    }
}

impl std::error::Error for EditError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Annotation {
    revision: u64,
    range: Range<usize>,
    original: String,
    instruction: String,
}

impl Annotation {
    pub fn original(&self) -> &str {
        &self.original
    }
    pub fn instruction(&self) -> &str {
        &self.instruction
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    annotation: usize,
    replacement: String,
    status: ProposalStatus,
}

impl Proposal {
    pub fn annotation_id(&self) -> usize {
        self.annotation
    }
    pub fn replacement(&self) -> &str {
        &self.replacement
    }
    pub fn status(&self) -> ProposalStatus {
        self.status
    }
}

/// Owns the only writable document and its review records. IDs are indexes local
/// to this workbench; no public API accepts a foreign proposal or annotation.
#[derive(Clone, Debug, Default)]
pub struct Workbench {
    text: String,
    revision: u64,
    annotations: Vec<Annotation>,
    proposals: Vec<Proposal>,
    history: Vec<String>,
    threads: Vec<CommentThread>,
}

/// Versioned wire DTO; deserialization alone grants no editing authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkbenchSnapshot {
    text: String,
    revision: u64,
    annotations: Vec<Annotation>,
    proposals: Vec<Proposal>,
    history: Vec<String>,
    #[serde(default)]
    threads: Vec<CommentThread>,
}

impl Workbench {
    pub fn snapshot(&self) -> WorkbenchSnapshot {
        WorkbenchSnapshot {
            text: self.text.clone(),
            revision: self.revision,
            annotations: self.annotations.clone(),
            proposals: self.proposals.clone(),
            history: self.history.clone(),
            threads: self.threads.clone(),
        }
    }

    /// Restores review records, not approval commands. Pending proposals still
    /// need a fresh explicit human accept; resolved proposals remain consumed.
    pub fn restore(state: WorkbenchSnapshot) -> Result<Self, EditError> {
        Self::check_size(state.text.len())?;
        if state.annotations.len() > MAX_RECORDS
            || state.proposals.len() > MAX_RECORDS
            || state.history.len() > MAX_HISTORY
            || state.history.len() as u64 > state.revision
            || state.revision == u64::MAX
        {
            return Err(EditError::InvalidSnapshot);
        }
        for target in &state.annotations {
            if target.revision > state.revision
                || target.range.start >= target.range.end
                || target.range.end > MAX_DOCUMENT_BYTES
                || target.original.len() != target.range.end - target.range.start
                || target.instruction.trim().is_empty()
            {
                return Err(EditError::InvalidSnapshot);
            }
            Self::check_size(target.original.len())?;
            Self::check_size(target.instruction.len())?;
            if target.revision == state.revision
                && state.text.get(target.range.clone()) != Some(target.original.as_str())
            {
                return Err(EditError::InvalidSnapshot);
            }
        }
        for proposal in &state.proposals {
            if proposal.annotation >= state.annotations.len() {
                return Err(EditError::InvalidSnapshot);
            }
            Self::check_size(proposal.replacement.len())?;
        }
        for text in &state.history {
            Self::check_size(text.len())?;
        }
        comments::validate_threads(&state.threads, &state.text, state.revision)?;
        let value = Self {
            text: state.text,
            revision: state.revision,
            annotations: state.annotations,
            proposals: state.proposals,
            history: state.history,
            threads: state.threads,
        };
        if value.state_bytes() > MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        Ok(value)
    }

    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }
    pub fn proposal_count(&self) -> usize {
        self.proposals.len()
    }

    fn state_bytes(&self) -> usize {
        self.text.len()
            + self
                .annotations
                .iter()
                .map(|a| a.original.len() + a.instruction.len())
                .sum::<usize>()
            + self
                .proposals
                .iter()
                .map(|p| p.replacement.len())
                .sum::<usize>()
            + self.history.iter().map(String::len).sum::<usize>()
            + self
                .threads
                .iter()
                .map(|t| {
                    t.original.len()
                        + t.pending.as_ref().map_or(0, |p| p.bytes())
                        + t.last_change
                            .as_ref()
                            .map_or(0, |c| c.before.len() + c.after.len())
                        + t.messages
                            .iter()
                            .map(|m| m.text.len() + m.author.as_ref().map_or(0, String::len))
                            .sum::<usize>()
                })
                .sum::<usize>()
    }

    pub fn new(text: impl Into<String>) -> Result<Self, EditError> {
        let text = text.into();
        Self::check_size(text.len())?;
        Ok(Self {
            text,
            ..Self::default()
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn annotation(&self, id: usize) -> Option<&Annotation> {
        self.annotations.get(id)
    }
    pub fn proposal(&self, id: usize) -> Option<&Proposal> {
        self.proposals.get(id)
    }

    /// Manual edits participate in the same revision and undo history as
    /// approved proposals. Setting identical text is a no-op.
    pub fn edit(&mut self, text: impl Into<String>) -> Result<(), EditError> {
        let text = text.into();
        Self::check_size(text.len())?;
        if text == self.text {
            return Ok(());
        }
        self.commit(text)
    }

    pub fn annotate(&mut self, range: Range<usize>, instruction: &str) -> Result<usize, EditError> {
        if range.start >= range.end {
            return Err(EditError::InvalidSelection);
        }
        let original = self
            .text
            .get(range.clone())
            .ok_or(EditError::InvalidSelection)?;
        if instruction.trim().is_empty() {
            return Err(EditError::EmptyInstruction);
        }
        Self::check_size(instruction.len())?;
        if self.annotations.len() >= MAX_RECORDS
            || self.state_bytes() + original.len() + instruction.len() > MAX_STATE_BYTES
        {
            return Err(EditError::StateLimit);
        }
        let id = self.annotations.len();
        self.annotations.push(Annotation {
            revision: self.revision,
            range,
            original: original.to_owned(),
            instruction: instruction.trim().to_owned(),
        });
        Ok(id)
    }

    /// Stages output only. Calling this never changes the document.
    pub fn propose(&mut self, annotation: usize, replacement: &str) -> Result<usize, EditError> {
        Self::check_size(replacement.len())?;
        let target = self
            .annotations
            .get(annotation)
            .ok_or(EditError::UnknownAnnotation)?;
        self.validate_target(target)?;
        if self.proposals.len() >= MAX_RECORDS
            || self.state_bytes() + replacement.len() > MAX_STATE_BYTES
        {
            return Err(EditError::StateLimit);
        }
        let id = self.proposals.len();
        self.proposals.push(Proposal {
            annotation,
            replacement: replacement.to_owned(),
            status: ProposalStatus::Pending,
        });
        Ok(id)
    }

    /// This is the explicit human-confirmation command, not a model tool.
    /// Checks and write are synchronous; failed checks never partially write.
    pub fn accept(&mut self, id: usize) -> Result<(), EditError> {
        let proposal = self.pending(id)?;
        let target = &self.annotations[proposal.annotation];
        if self.validate_target(target).is_err() {
            self.proposals[id].status = ProposalStatus::Conflicted;
            return Err(EditError::Conflict);
        }
        let size = self.text.len() - target.original.len() + proposal.replacement.len();
        Self::check_size(size)?;
        let mut next = self.text.clone();
        next.replace_range(target.range.clone(), &proposal.replacement);
        // A no-op acceptance still advances the revision and consumes approval.
        self.commit(next)?;
        self.proposals[id].status = ProposalStatus::Applied;
        Ok(())
    }

    pub fn reject(&mut self, id: usize) -> Result<(), EditError> {
        self.pending(id)?;
        self.proposals[id].status = ProposalStatus::Rejected;
        Ok(())
    }

    /// Replace the entire document text in one trusted write. Used by
    /// internal surfaces (e.g. the title input) that already have validated
    /// the new string and don't need the proposal/review flow. Revision and
    /// undo history are advanced exactly like a `commit`.
    pub fn set_text(&mut self, text: String) -> Result<(), EditError> {
        Self::check_size(text.len())?;
        self.commit(text)?;
        Ok(())
    }
    /// Undo restores content but never rewinds revision numbers. Old proposals
    /// stay resolved, so restoring old text cannot replay an old approval.
    pub fn undo(&mut self) -> Result<(), EditError> {
        let revision = self.next_revision()?;
        let previous = self.history.pop().ok_or(EditError::NothingToUndo)?;
        self.text = previous;
        self.revision = revision;
        Ok(())
    }

    fn pending(&self, id: usize) -> Result<&Proposal, EditError> {
        let proposal = self.proposals.get(id).ok_or(EditError::UnknownProposal)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(EditError::ProposalAlreadyResolved);
        }
        Ok(proposal)
    }

    fn validate_target(&self, target: &Annotation) -> Result<(), EditError> {
        if target.revision != self.revision
            || self.text.get(target.range.clone()) != Some(target.original.as_str())
        {
            return Err(EditError::Conflict);
        }
        Ok(())
    }

    fn check_size(bytes: usize) -> Result<(), EditError> {
        if bytes > MAX_DOCUMENT_BYTES {
            return Err(EditError::DocumentTooLarge);
        }
        Ok(())
    }

    fn next_revision(&self) -> Result<u64, EditError> {
        self.revision
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)
    }

    fn commit(&mut self, text: String) -> Result<(), EditError> {
        let revision = self.next_revision()?;
        // Old history may be evicted, but never drop review records silently.
        let mut bytes = self.state_bytes() + text.len();
        let mut evict = usize::from(self.history.len() == MAX_HISTORY);
        if evict > 0 {
            bytes -= self.history[0].len();
        }
        while bytes > MAX_STATE_BYTES && evict < self.history.len() {
            bytes -= self.history[evict].len();
            evict += 1;
        }
        if bytes > MAX_STATE_BYTES {
            return Err(EditError::StateLimit);
        }
        self.relocate_threads(&text, revision);
        self.history.drain(..evict);
        self.history.push(std::mem::replace(&mut self.text, text));
        self.revision = revision;
        Ok(())
    }
}
