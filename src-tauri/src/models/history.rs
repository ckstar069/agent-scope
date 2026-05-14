// Claude history related structs are re-exported from collectors::claude_history::models
// for a unified models module interface.
pub use crate::collectors::claude_history::models::{
    ExportFormat, SerClaudeSession, SerHistoryEntry, SerPreviewMessage, SerProjectSessionGroup,
    SerSessionPreview, SerSessionStatus,
};
