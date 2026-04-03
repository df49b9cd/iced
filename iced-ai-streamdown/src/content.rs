use iced::widget::markdown;

use crate::remend::RemendOptions;

/// A streaming markdown document that tracks changes across incremental updates.
///
/// Wraps [`markdown::Content`] and maintains word counts per block for
/// animation synchronization. Optionally preprocesses streaming text with
/// [`remend`](crate::remend) to auto-complete incomplete markdown syntax.
#[derive(Debug)]
pub struct StreamContent {
    inner: markdown::Content,
    previous_item_count: usize,
    is_streaming: bool,
    /// Word count per block (indexed by block index).
    word_counts: Vec<usize>,
    /// Cumulative word count prefix sums: `cumulative_words[i]` is the
    /// total words in blocks `0..i`.
    cumulative_words: Vec<usize>,
    /// Accumulated raw markdown input (used when remend is enabled).
    raw_markdown: String,
    /// When `Some`, incomplete markdown syntax is auto-completed before parsing.
    remend_options: Option<RemendOptions>,
}

impl Default for StreamContent {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamContent {
    /// Creates a new empty [`StreamContent`].
    pub fn new() -> Self {
        Self {
            inner: markdown::Content::new(),
            previous_item_count: 0,
            is_streaming: false,
            word_counts: Vec::new(),
            cumulative_words: vec![0],
            raw_markdown: String::new(),
            remend_options: None,
        }
    }

    /// Creates a new [`StreamContent`] with remend preprocessing enabled.
    ///
    /// Incomplete markdown syntax will be auto-completed before parsing,
    /// ensuring content renders correctly during token-by-token streaming.
    pub fn with_remend(options: RemendOptions) -> Self {
        Self {
            remend_options: Some(options),
            ..Self::new()
        }
    }

    /// Sets or clears the remend preprocessing options.
    pub fn set_remend_options(&mut self, options: Option<RemendOptions>) {
        self.remend_options = options;
    }

    /// Returns the current remend options, if enabled.
    pub fn remend_options(&self) -> Option<&RemendOptions> {
        self.remend_options.as_ref()
    }

    /// Pushes more markdown text into the content, parsing incrementally.
    ///
    /// When remend is enabled, the full accumulated text is preprocessed
    /// and re-parsed to handle retroactive syntax completion (e.g., closing
    /// a `**` that was opened hundreds of characters ago).
    ///
    /// Automatically enters streaming mode on the first call. Call
    /// [`finish`](Self::finish) when the stream is complete.
    pub fn push_str(&mut self, markdown: &str) {
        if !self.is_streaming {
            self.is_streaming = true;
        }

        self.previous_item_count = self.inner.items().len();

        if let Some(ref options) = self.remend_options {
            self.raw_markdown.push_str(markdown);
            let processed = crate::remend::remend(&self.raw_markdown, options);
            // Must re-create Content since remend may change the full output.
            self.inner = markdown::Content::new();
            self.inner.push_str(&processed);
        } else {
            self.inner.push_str(markdown);
        }

        self.recompute_word_counts();
    }

    /// Marks the stream as finished.
    ///
    /// When remend is enabled, performs a final re-parse of the raw markdown
    /// *without* remend preprocessing (since the complete text should have
    /// valid syntax).
    pub fn finish(&mut self) {
        if self.remend_options.is_some() && !self.raw_markdown.is_empty() {
            // Final parse: use the raw markdown as-is (complete text).
            self.inner = markdown::Content::new();
            self.inner.push_str(&self.raw_markdown);
            self.recompute_word_counts();
        }
        self.is_streaming = false;
    }

    /// Returns whether content is currently being streamed.
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }

    /// Returns the parsed markdown items.
    pub fn items(&self) -> &[markdown::Item] {
        self.inner.items()
    }

    /// Returns the inner [`markdown::Content`].
    pub fn inner(&self) -> &markdown::Content {
        &self.inner
    }

    /// Returns the total number of words across all blocks.
    pub fn total_word_count(&self) -> usize {
        self.cumulative_words.last().copied().unwrap_or(0)
    }

    /// Returns the global word index where the given block starts.
    pub fn global_word_offset(&self, block_index: usize) -> usize {
        self.cumulative_words
            .get(block_index)
            .copied()
            .unwrap_or(0)
    }

    /// Returns the number of blocks that existed before the last `push_str`.
    pub fn previous_item_count(&self) -> usize {
        self.previous_item_count
    }

    /// Recomputes word counts for blocks that may have changed.
    ///
    /// Only recounts from the last unchanged block onward.
    fn recompute_word_counts(&mut self) {
        let items = self.inner.items();
        let recount_from = if self.previous_item_count > 0 {
            self.previous_item_count - 1
        } else {
            0
        };

        self.word_counts.truncate(recount_from);
        self.cumulative_words.truncate(recount_from + 1);
        if self.cumulative_words.is_empty() {
            self.cumulative_words.push(0);
        }

        for item in items.iter().skip(recount_from) {
            let count = count_words_in_item(item);
            self.word_counts.push(count);
            let prev = *self.cumulative_words.last().unwrap();
            self.cumulative_words.push(prev + count);
        }
    }
}

/// Public wrapper for word counting in items (used by view.rs for offset tracking).
pub fn count_words_in_item_public(item: &markdown::Item) -> usize {
    count_words_in_item(item)
}

/// Counts words in a markdown item by examining its text content.
fn count_words_in_item(item: &markdown::Item) -> usize {
    match item {
        markdown::Item::Heading(_, text) => count_words_in_text(text),
        markdown::Item::Paragraph(text) => count_words_in_text(text),
        markdown::Item::CodeBlock { code, .. } => code.split_whitespace().count(),
        markdown::Item::List { bullets, .. } => {
            bullets.iter().map(count_words_in_bullet).sum()
        }
        markdown::Item::Quote(items) => items.iter().map(count_words_in_item).sum(),
        markdown::Item::Image { title, .. } => title.split_whitespace().count(),
        markdown::Item::Rule => 0,
        markdown::Item::Table { columns, rows } => {
            // Count header words from column headers.
            let header_words: usize = columns
                .iter()
                .flat_map(|col| &col.header)
                .map(count_words_in_item)
                .sum();
            // Row cells are not publicly accessible, so estimate based
            // on the number of rows and columns (roughly 2 words per cell).
            let row_words = rows.len() * columns.len() * 2;
            header_words + row_words
        }
    }
}

fn count_words_in_bullet(bullet: &markdown::Bullet) -> usize {
    let items = match bullet {
        markdown::Bullet::Point { items } => items,
        markdown::Bullet::Task { items, .. } => items,
    };
    items.iter().map(count_words_in_item).sum()
}

/// Counts words in a [`markdown::Text`] by getting its raw span text content.
///
/// We use the spans API with a dummy style since we only need the text content.
/// The style doesn't affect the text itself, just the formatting.
fn count_words_in_text(text: &markdown::Text) -> usize {
    // We access the text via the public spans API.
    // Use a default style — the text content is style-independent.
    use iced::theme::palette::Seed;
    let style = markdown::Style::from_palette(Seed::CATPPUCCIN_MOCHA);
    let spans = text.spans(style);
    spans
        .iter()
        .map(|span| {
            let s: &str = span.text.as_ref();
            s.split_whitespace().count()
        })
        .sum()
}
