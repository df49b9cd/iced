use std::borrow::Cow;

/// How to handle incomplete links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkMode {
    /// Use `streamdown:incomplete-link` placeholder URL (default).
    #[default]
    Protocol,
    /// Display only the link text without any link markup.
    TextOnly,
}

/// A custom handler that transforms text during the remend pipeline.
///
/// Implement this trait to add custom preprocessing steps. Custom handlers
/// are merged with the built-in handlers and sorted by priority.
pub trait RemendHandler: Send + Sync {
    /// Transform the text. Return `Cow::Borrowed(text)` if no changes are needed.
    fn handle<'a>(&self, text: &'a str) -> Cow<'a, str>;

    /// Unique identifier for this handler.
    fn name(&self) -> &str;

    /// Priority (lower runs first). Built-in priorities use 0–75.
    /// Custom handlers default to 100.
    fn priority(&self) -> i32 {
        100
    }
}

/// Built-in handler priorities, matching the TypeScript implementation.
pub mod priority {
    pub const SINGLE_TILDE: i32 = 0;
    pub const COMPARISON_OPERATORS: i32 = 5;
    pub const HTML_TAGS: i32 = 10;
    pub const SETEXT_HEADINGS: i32 = 15;
    pub const LINKS: i32 = 20;
    pub const BOLD_ITALIC: i32 = 30;
    pub const BOLD: i32 = 35;
    pub const ITALIC_DOUBLE_UNDERSCORE: i32 = 40;
    pub const ITALIC_SINGLE_ASTERISK: i32 = 41;
    pub const ITALIC_SINGLE_UNDERSCORE: i32 = 42;
    pub const INLINE_CODE: i32 = 50;
    pub const STRIKETHROUGH: i32 = 60;
    pub const KATEX: i32 = 70;
    pub const INLINE_KATEX: i32 = 75;
    pub const DEFAULT: i32 = 100;
}

/// Configuration options for the [`remend`](super::remend) function.
///
/// All options default to `true` (enabled) except `inline_katex` which
/// defaults to `false` (single `$` is ambiguous with currency symbols).
pub struct RemendOptions {
    /// Complete bold formatting (`**text` → `**text**`).
    pub bold: bool,
    /// Complete italic formatting (`*text` → `*text*`, `_text` → `_text_`).
    pub italic: bool,
    /// Complete bold-italic formatting (`***text` → `***text***`).
    pub bold_italic: bool,
    /// Complete inline code formatting (`` `code `` → `` `code` ``).
    pub inline_code: bool,
    /// Complete strikethrough formatting (`~~text` → `~~text~~`).
    pub strikethrough: bool,
    /// Complete links (`[text](url` → `[text](streamdown:incomplete-link)`).
    pub links: bool,
    /// Handle incomplete images (`![alt](url` → removed).
    pub images: bool,
    /// Complete block KaTeX math (`$$eq` → `$$eq$$`).
    pub katex: bool,
    /// Complete inline KaTeX math (`$eq` → `$eq$`).
    /// Defaults to `false` — single `$` is ambiguous with currency symbols.
    pub inline_katex: bool,
    /// Handle incomplete setext headings to prevent misinterpretation.
    pub setext_headings: bool,
    /// Strip incomplete HTML tags at end of text.
    pub html_tags: bool,
    /// Escape single `~` between word characters.
    pub single_tilde: bool,
    /// Escape `>` as comparison operators in list items.
    pub comparison_operators: bool,
    /// How to handle incomplete links.
    pub link_mode: LinkMode,
    /// Custom handlers to extend the remend pipeline.
    pub handlers: Vec<Box<dyn RemendHandler>>,
}

impl std::fmt::Debug for RemendOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemendOptions")
            .field("bold", &self.bold)
            .field("italic", &self.italic)
            .field("bold_italic", &self.bold_italic)
            .field("inline_code", &self.inline_code)
            .field("strikethrough", &self.strikethrough)
            .field("links", &self.links)
            .field("images", &self.images)
            .field("katex", &self.katex)
            .field("inline_katex", &self.inline_katex)
            .field("setext_headings", &self.setext_headings)
            .field("html_tags", &self.html_tags)
            .field("single_tilde", &self.single_tilde)
            .field("comparison_operators", &self.comparison_operators)
            .field("link_mode", &self.link_mode)
            .field("handlers", &format!("[{} custom]", self.handlers.len()))
            .finish()
    }
}

impl Default for RemendOptions {
    fn default() -> Self {
        Self {
            bold: true,
            italic: true,
            bold_italic: true,
            inline_code: true,
            strikethrough: true,
            links: true,
            images: true,
            katex: true,
            inline_katex: false,
            setext_headings: true,
            html_tags: true,
            single_tilde: true,
            comparison_operators: true,
            link_mode: LinkMode::Protocol,
            handlers: Vec::new(),
        }
    }
}

impl RemendOptions {
    pub fn bold(mut self, enabled: bool) -> Self {
        self.bold = enabled;
        self
    }

    pub fn italic(mut self, enabled: bool) -> Self {
        self.italic = enabled;
        self
    }

    pub fn bold_italic(mut self, enabled: bool) -> Self {
        self.bold_italic = enabled;
        self
    }

    pub fn inline_code(mut self, enabled: bool) -> Self {
        self.inline_code = enabled;
        self
    }

    pub fn strikethrough(mut self, enabled: bool) -> Self {
        self.strikethrough = enabled;
        self
    }

    pub fn links(mut self, enabled: bool) -> Self {
        self.links = enabled;
        self
    }

    pub fn images(mut self, enabled: bool) -> Self {
        self.images = enabled;
        self
    }

    pub fn katex(mut self, enabled: bool) -> Self {
        self.katex = enabled;
        self
    }

    pub fn setext_headings(mut self, enabled: bool) -> Self {
        self.setext_headings = enabled;
        self
    }

    pub fn html_tags(mut self, enabled: bool) -> Self {
        self.html_tags = enabled;
        self
    }

    pub fn single_tilde(mut self, enabled: bool) -> Self {
        self.single_tilde = enabled;
        self
    }

    pub fn comparison_operators(mut self, enabled: bool) -> Self {
        self.comparison_operators = enabled;
        self
    }

    pub fn inline_katex(mut self, enabled: bool) -> Self {
        self.inline_katex = enabled;
        self
    }

    pub fn link_mode(mut self, mode: LinkMode) -> Self {
        self.link_mode = mode;
        self
    }

    /// Add a custom handler to the pipeline.
    pub fn handler(mut self, handler: Box<dyn RemendHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
}
