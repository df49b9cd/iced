/// How to handle incomplete links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkMode {
    /// Use `streamdown:incomplete-link` placeholder URL (default).
    #[default]
    Protocol,
    /// Display only the link text without any link markup.
    TextOnly,
}

/// Configuration options for the [`remend`](super::remend) function.
///
/// All options default to `true` (enabled) except `inline_katex` which
/// defaults to `false` (single `$` is ambiguous with currency symbols).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}
