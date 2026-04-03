use iced::widget::markdown;

/// A streaming markdown document that tracks changes across incremental updates.
///
/// Wraps [`markdown::Content`] and maintains word counts per block for
/// animation synchronization.
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
        }
    }

    /// Pushes more markdown text into the content, parsing incrementally.
    ///
    /// Automatically enters streaming mode on the first call. Call
    /// [`finish`](Self::finish) when the stream is complete.
    pub fn push_str(&mut self, markdown: &str) {
        if !self.is_streaming {
            self.is_streaming = true;
        }

        self.previous_item_count = self.inner.items().len();
        self.inner.push_str(markdown);
        self.recompute_word_counts();
    }

    /// Marks the stream as finished.
    pub fn finish(&mut self) {
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
